//! A shell's process and what it runs (specs/16, "Process usage"), for a terminal to say.

use windows::Wdk::System::Threading::{NtQueryInformationProcess, ProcessCommandLineInformation};
use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, UNICODE_STRING};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};
use windows::core::PWSTR;

use crate::ShellProcess;

/// The console hosts a shell may start: not what it runs.
const HOSTS: [&str; 2] = ["conhost.exe", "openconsole.exe"];

/// The full path of `pid` and the command line of the newest process it started.
pub fn shell_process(pid: u32) -> Option<ShellProcess> {
    let shell = Opened::new(pid)?;
    let path = shell.path()?;
    let running = children(pid)
        .into_iter()
        .filter_map(Opened::new)
        .filter_map(|child| Some((child.created()?, child)))
        .max_by_key(|(created, _)| *created)
        .and_then(|(_, child)| child.command_line());
    Some(ShellProcess { path, running })
}

/// The processes `pid` started, but the console hosts.
fn children(pid: u32) -> Vec<u32> {
    let mut found = Vec::new();
    // SAFETY: the snapshot is a handle of ours, closed after the walk; the entry is a local of
    // the size it says.
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return found };
        let mut entry =
            PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut more = Process32FirstW(snapshot, &mut entry).is_ok();
        while more {
            if entry.th32ParentProcessID == pid && entry.th32ProcessID != pid {
                let end = entry.szExeFile.iter().position(|c| *c == 0).unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..end]).to_lowercase();
                if !HOSTS.contains(&name.as_str()) {
                    found.push(entry.th32ProcessID);
                }
            }
            more = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
    }
    found
}

/// A process opened to be read, closed when dropped.
struct Opened(HANDLE);

impl Opened {
    fn new(pid: u32) -> Option<Opened> {
        // SAFETY: a plain call; the handle is ours and closed on drop.
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok().map(Opened)
    }

    fn path(&self) -> Option<String> {
        let mut buffer = vec![0u16; 1024];
        let mut size = buffer.len() as u32;
        // SAFETY: the buffer holds `size` units, and the call writes no more.
        unsafe {
            QueryFullProcessImageNameW(self.0, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut size)
                .ok()?
        };
        Some(String::from_utf16_lossy(&buffer[..size as usize]))
    }

    /// When it was created, in 100 ns since 1601.
    fn created(&self) -> Option<u64> {
        let mut times = [FILETIME::default(); 4];
        let [created, exited, kernel, user] = &mut times;
        // SAFETY: each pointer is to a local FILETIME.
        unsafe { GetProcessTimes(self.0, created, exited, kernel, user).ok()? };
        Some(u64::from(times[0].dwHighDateTime) << 32 | u64::from(times[0].dwLowDateTime))
    }

    /// Its command line: a UNICODE_STRING at the start of the buffer, its text after it.
    fn command_line(&self) -> Option<String> {
        let mut buffer = vec![0u64; 4096];
        let bytes = (buffer.len() * 8) as u32;
        let mut needed = 0u32;
        // SAFETY: the buffer is `bytes` long and 8-aligned; the call writes no more, and the
        // string it writes points inside it.
        unsafe {
            let at = buffer.as_mut_ptr().cast::<core::ffi::c_void>();
            NtQueryInformationProcess(self.0, ProcessCommandLineInformation, at, bytes, &mut needed)
                .ok()
                .ok()?;
            let said = &*at.cast::<UNICODE_STRING>();
            if said.Buffer.is_null() || said.Length == 0 {
                return None;
            }
            let units = std::slice::from_raw_parts(said.Buffer.0, usize::from(said.Length / 2));
            Some(String::from_utf16_lossy(units).trim().to_string())
        }
    }
}

impl Drop for Opened {
    fn drop(&mut self) {
        // SAFETY: the handle is ours, closed once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[cfg(test)]
mod tests {
    /// This test's own process, a child of the one that ran it: its path is an `.exe`.
    #[test]
    fn a_process_s_path_is_read_and_a_gone_one_is_none() {
        let me = std::process::id();
        let found = super::shell_process(me).expect("this process");
        assert!(found.path.to_lowercase().ends_with(".exe"), "{}", found.path);
        assert!(super::shell_process(u32::MAX - 3).is_none());
    }

    #[test]
    fn a_shell_s_full_path_and_the_command_it_runs() {
        let mut shell = std::process::Command::new("cmd.exe")
            .args(["/c", "ping -n 3 127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let found = super::shell_process(shell.id());
        let _ = std::process::Command::new("taskkill")
            .args(["/f", "/t", "/pid", &shell.id().to_string()])
            .output();
        let _ = shell.wait();
        let found = found.expect("the shell");
        assert!(found.path.to_lowercase().ends_with(r"\cmd.exe"), "{}", found.path);
        let running = found.running.expect("ping");
        assert!(running.to_lowercase().contains("ping") && running.contains("-n 3 127.0.0.1"), "{running}");
    }
}
