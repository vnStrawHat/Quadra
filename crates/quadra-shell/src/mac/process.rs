//! A shell's process and what it runs (specs/16, "Process usage"), for a terminal to say;
//! and the environment a new shell is given.

use std::ffi::{c_int, c_void};

use super::usage::used;
use crate::ShellProcess;

/// `PROC_PPID_ONLY`: the processes a process started.
const STARTED_BY: u32 = 6;
/// `CTL_KERN` and `KERN_PROCARGS2`: the arguments a process was started with.
const ARGUMENTS: [c_int; 2] = [1, 49];
/// The most a process's arguments and environment take (`ARG_MAX`).
const MOST: usize = 1 << 20;

unsafe extern "C" {
    fn proc_pidpath(pid: c_int, buffer: *mut c_void, size: u32) -> c_int;
    fn proc_listpids(kind: u32, of: u32, buffer: *mut c_void, size: c_int) -> c_int;
    fn sysctl(
        name: *mut c_int,
        names: u32,
        old: *mut c_void,
        old_len: *mut usize,
        new: *mut c_void,
        new_len: usize,
    ) -> c_int;
}

/// What a new shell is given. macOS keeps no user's environment beside the processes: a
/// shell makes its own from its profiles, over the app's.
pub fn user_environment() -> Vec<(String, String)> {
    let text = |it: std::ffi::OsString| it.to_string_lossy().into_owned();
    std::env::vars_os().map(|(name, value)| (text(name), text(value))).collect()
}

/// The full path of `pid` and the command line of the newest process it started.
pub fn shell_process(pid: u32) -> Option<ShellProcess> {
    let path = path(pid)?;
    let running = children(pid)
        .into_iter()
        .filter_map(|child| Some((used(child)?.start, child)))
        .max_by_key(|(started, _)| *started)
        .and_then(|(_, child)| command_line(child));
    Some(ShellProcess { path, running })
}

fn path(pid: u32) -> Option<String> {
    let mut out = vec![0u8; 4096];
    // SAFETY: the buffer is told with its length; what is written is a path of that many bytes.
    let len = unsafe { proc_pidpath(pid as c_int, out.as_mut_ptr().cast(), out.len() as u32) };
    (len > 0).then(|| String::from_utf8_lossy(&out[..len as usize]).into_owned())
}

/// The processes `pid` started.
fn children(pid: u32) -> Vec<u32> {
    let mut found = vec![0u32; 1024];
    let size = (found.len() * size_of::<u32>()) as c_int;
    // SAFETY: the buffer is told with its size in bytes; the answer is how many were written.
    let written = unsafe { proc_listpids(STARTED_BY, pid, found.as_mut_ptr().cast(), size) };
    found.truncate(written.max(0) as usize / size_of::<u32>());
    found.retain(|&it| it != 0 && it != pid);
    found
}

/// The arguments `pid` was started with, a space between two.
/// ponytail: an argument with a space in it reads as two; quoted when a reader takes them
/// apart again.
fn command_line(pid: u32) -> Option<String> {
    let mut name = [ARGUMENTS[0], ARGUMENTS[1], pid as c_int];
    let (mut out, mut len) = (vec![0u8; MOST], MOST);
    // SAFETY: the name is three numbers, the buffer is told with its length, which the call
    // sets to what it wrote.
    let status =
        unsafe { sysctl(name.as_mut_ptr(), 3, out.as_mut_ptr().cast(), &mut len, std::ptr::null_mut(), 0) };
    if status != 0 || len < 4 {
        return None;
    }
    // How many arguments, the program's path and zeros after it, then the arguments, each
    // ended by a zero; the environment follows them.
    let count = i32::from_ne_bytes([out[0], out[1], out[2], out[3]]).max(0) as usize;
    let rest = &out[4..len.min(MOST)];
    let after_path = rest.iter().position(|&b| b == 0)?;
    let rest = &rest[after_path..];
    let rest = &rest[rest.iter().position(|&b| b != 0)?..];
    let args: Vec<String> =
        rest.split(|&b| b == 0).take(count).map(|it| String::from_utf8_lossy(it).into_owned()).collect();
    (!args.is_empty()).then(|| args.join(" "))
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::time::Duration;

    use super::*;

    #[test]
    fn a_process_s_path_is_read_and_a_gone_one_is_none() {
        let found = shell_process(std::process::id()).expect("this process");
        assert!(found.path.starts_with('/'), "{}", found.path);
        assert!(shell_process(u32::MAX - 3).is_none());
        assert!(user_environment().iter().any(|(name, _)| name == "PATH"));
    }

    #[test]
    fn a_shell_s_full_path_and_the_command_it_runs() {
        let mut shell = Command::new("/bin/sh").args(["-c", "sleep 3; true"]).spawn().unwrap();
        std::thread::sleep(Duration::from_millis(300));
        let found = shell_process(shell.id());
        let _ = shell.kill();
        let _ = shell.wait();
        let found = found.expect("the shell");
        assert!(found.path.starts_with("/bin/"), "{}", found.path);
        assert_eq!(found.running.as_deref(), Some("sleep 3"));
    }
}
