//! One instance (specs/16, "One instance"): the first process of an app holds a named mutex
//! and a hidden window; a later one hands that window its arguments and ends.

use std::path::PathBuf;

use quadra_async::{Sender, inbox};
use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, FindWindowExW, GWLP_USERDATA, GetWindowLongPtrW, HWND_MESSAGE,
    RegisterClassExW, SendMessageW, SetWindowLongPtrW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_COPYDATA,
    WNDCLASSEXW,
};
use windows::core::{HSTRING, w};

use crate::{Instance, Launch};

/// The mark of what this module sends, so a message of another kind is not read as one.
const LAUNCH: usize = 0x5155_4144;

unsafe extern "system" fn listen(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: the window's data is the sender `single_instance` gave it, which lives as long
    // as the process; WM_COPYDATA's lparam is a structure the OS copied for this call.
    unsafe {
        let sender = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Sender<Launch>;
        if msg != WM_COPYDATA || sender.is_null() {
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        let data = &*(lparam.0 as *const COPYDATASTRUCT);
        if data.dwData != LAUNCH || data.lpData.is_null() {
            return LRESULT(0);
        }
        let units = std::slice::from_raw_parts(data.lpData as *const u16, data.cbData as usize / 2);
        let text = String::from_utf16_lossy(units);
        let mut parts = text.split('\0').map(str::to_string);
        let folder = PathBuf::from(parts.next().unwrap_or_default());
        (*sender).push(Launch { folder, args: parts.collect() });
        LRESULT(1)
    }
}

/// How long a later process looks for the first's window, which the first makes just after
/// its mutex: the two may start together.
const WAIT: std::time::Duration = std::time::Duration::from_secs(2);

/// Whether this process is the first of the app named `id` in the user's session; a later
/// one hands the first `args` and the folder it was started in.
pub fn single_instance(id: &str, args: &[String]) -> Instance {
    let (name, class) = (HSTRING::from(format!("Local\\quadra-{id}")), w!("QuadraInstance"));
    let title = HSTRING::from(id);
    // SAFETY: plain calls with strings that live through them. The mutex is never closed:
    // it is the process's until it ends.
    unsafe {
        let mutex = CreateMutexW(None, true, &name);
        if mutex.is_ok() && GetLastError() == ERROR_ALREADY_EXISTS {
            // The user started this one: the first may take the foreground from it.
            let _ = windows::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(u32::MAX);
            let started = std::time::Instant::now();
            let mut found = FindWindowExW(Some(HWND_MESSAGE), None, class, &title);
            while found.is_err() && started.elapsed() < WAIT {
                std::thread::sleep(std::time::Duration::from_millis(20));
                found = FindWindowExW(Some(HWND_MESSAGE), None, class, &title);
            }
            if let Ok(first) = found {
                let folder = std::env::current_dir().unwrap_or_default();
                let mut text = folder.display().to_string();
                args.iter().for_each(|arg| text.extend(['\0'].into_iter().chain(arg.chars())));
                let units: Vec<u16> = text.encode_utf16().collect();
                let data = COPYDATASTRUCT {
                    dwData: LAUNCH,
                    cbData: (units.len() * 2) as u32,
                    lpData: units.as_ptr() as *mut _,
                };
                SendMessageW(first, WM_COPYDATA, None, Some(LPARAM(&data as *const _ as isize)));
            }
            return Instance::Second;
        }
        let launches = inbox();
        let instance = GetModuleHandleW(None).ok();
        let window_class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(listen),
            hInstance: instance.map(Into::into).unwrap_or_default(),
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassExW(&window_class); // a second register fails, and changes nothing
        let style = (WINDOW_EX_STYLE(0), WINDOW_STYLE(0));
        let made = CreateWindowExW(
            style.0,
            class,
            &title,
            style.1,
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            instance.map(Into::into),
            None,
        );
        if let Ok(hwnd) = made {
            let sender = Box::into_raw(Box::new(launches.sender()));
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, sender as isize);
        }
        Instance::First(launches)
    }
}
