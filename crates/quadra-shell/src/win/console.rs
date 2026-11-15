//! The console a program of the bench writes to (specs/16, "Console").

use std::io::Write;

use windows::Win32::Foundation::HANDLE;
use windows::Win32::Globalization::CP_UTF8;
use windows::Win32::System::Console::{
    CONSOLE_MODE, CONSOLE_SCREEN_BUFFER_INFO, DISABLE_NEWLINE_AUTO_RETURN, ENABLE_PROCESSED_OUTPUT,
    ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetConsoleScreenBufferInfo, GetStdHandle,
    STD_OUTPUT_HANDLE, SetConsoleMode, SetConsoleOutputCP, WriteConsoleA,
};

fn output() -> Option<HANDLE> {
    // SAFETY: asks for the process's standard output; nothing is written through it here.
    unsafe { GetStdHandle(STD_OUTPUT_HANDLE) }.ok().filter(|it| !it.is_invalid())
}

/// The console window's columns and rows; None when the output is no console.
pub fn size() -> Option<(u16, u16)> {
    let mut info = CONSOLE_SCREEN_BUFFER_INFO::default();
    // SAFETY: `info` is a local of the size the call writes.
    unsafe { GetConsoleScreenBufferInfo(output()?, &mut info) }.ok()?;
    let window = info.srWindow;
    Some(((window.Right - window.Left + 1) as u16, (window.Bottom - window.Top + 1) as u16))
}

/// Virtual terminal processing on the output, no return after a line feed, and UTF-8; false
/// when the output is no console.
pub fn enable_vt() -> bool {
    let Some(out) = output() else { return false };
    let mut mode = CONSOLE_MODE::default();
    // SAFETY: `mode` is a local the call writes; the handle is the process's output.
    unsafe {
        if GetConsoleMode(out, &mut mode).is_err() {
            return false;
        }
        let mode =
            mode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING | DISABLE_NEWLINE_AUTO_RETURN;
        SetConsoleMode(out, mode).is_ok() && SetConsoleOutputCP(CP_UTF8).is_ok()
    }
}

/// Writes all of `bytes` to the console in one call where it can (`WriteConsoleA`), or to the
/// output as it is when that is no console.
pub fn write(bytes: &[u8]) {
    let mut rest = bytes;
    while !rest.is_empty() {
        let mut written = 0u32;
        // SAFETY: the buffer is the slice's own, its length told; `written` is a local.
        let ok =
            output().is_some_and(|out| unsafe { WriteConsoleA(out, rest, Some(&mut written), None) }.is_ok());
        if !ok || written == 0 {
            let mut out = std::io::stdout().lock();
            let _ = out.write_all(rest).and_then(|_| out.flush());
            return;
        }
        rest = &rest[(written as usize).min(rest.len())..];
    }
}
