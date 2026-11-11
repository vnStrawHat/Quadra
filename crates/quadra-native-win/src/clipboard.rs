//! The Windows clipboard, as Unicode text. No test runs this: tests never touch the real
//! clipboard (the headless platform has one in memory).

use quadra_platform::Clipboard;
use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock};

const CF_UNICODETEXT: u32 = 13;

/// The clipboard, opened in the name of a window of this thread.
pub(crate) struct WinClipboard(pub isize);

/// Closes the clipboard when the call that opened it is over.
struct Open;

impl Open {
    fn new(owner: isize) -> Option<Open> {
        // SAFETY: opening the clipboard for a window of this thread; it is closed by `Drop`.
        unsafe { OpenClipboard(Some(HWND(owner as *mut _))) }.ok().map(|()| Open)
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        // SAFETY: this thread opened the clipboard.
        let _ = unsafe { CloseClipboard() };
    }
}

impl Clipboard for WinClipboard {
    fn read_text(&self) -> Option<String> {
        let _open = Open::new(self.0)?;
        // SAFETY: the clipboard is open. Its text is a zero-terminated UTF-16 string in a
        // global block that is at least its size long and stays locked while it is read.
        unsafe {
            let block = HGLOBAL(GetClipboardData(CF_UNICODETEXT).ok()?.0);
            let text = GlobalLock(block).cast::<u16>();
            if text.is_null() {
                return None;
            }
            let units = std::slice::from_raw_parts(text, GlobalSize(block) / 2);
            let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
            let read = String::from_utf16_lossy(&units[..end]);
            let _ = GlobalUnlock(block);
            Some(read.replace("\r\n", "\n"))
        }
    }

    fn write_text(&self, text: &str) {
        let units: Vec<u16> = text.replace('\n', "\r\n").encode_utf16().chain([0]).collect();
        let Some(_open) = Open::new(self.0) else { return };
        // SAFETY: the clipboard is open. The block is as long as the units copied into it;
        // once the clipboard has it, the system owns it.
        unsafe {
            let _ = EmptyClipboard();
            let Ok(block) = GlobalAlloc(GMEM_MOVEABLE, units.len() * 2) else { return };
            let to = GlobalLock(block).cast::<u16>();
            if to.is_null() {
                return;
            }
            std::ptr::copy_nonoverlapping(units.as_ptr(), to, units.len());
            let _ = GlobalUnlock(block);
            let _ = SetClipboardData(CF_UNICODETEXT, Some(HANDLE(block.0)));
        }
    }
}
