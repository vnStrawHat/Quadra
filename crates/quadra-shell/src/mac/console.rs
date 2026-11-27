//! The console a program of the bench writes to (specs/16, "Console"): a terminal of macOS
//! reads the sequences as they are, in UTF-8.

use std::ffi::{c_int, c_ulong};
use std::io::{IsTerminal, Write};

/// `TIOCGWINSZ`: asks a terminal its size.
const SIZE_OF_WINDOW: c_ulong = 0x4008_7468;

unsafe extern "C" {
    fn ioctl(file: c_int, request: c_ulong, ...) -> c_int;
}

/// The terminal's columns and rows; None when the output is no terminal.
pub fn size() -> Option<(u16, u16)> {
    // Rows, columns, and the two sizes in pixels.
    let mut size = [0u16; 4];
    // SAFETY: the request writes four numbers of sixteen bits where it is pointed.
    let status = unsafe { ioctl(1, SIZE_OF_WINDOW, size.as_mut_ptr()) };
    (status == 0 && size[1] > 0).then_some((size[1], size[0]))
}

/// Whether the output reads the sequences: a terminal does, with nothing to turn on.
pub fn enable_vt() -> bool {
    std::io::stdout().is_terminal()
}

/// Writes all of `bytes` to the output, at once.
pub fn write(bytes: &[u8]) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(bytes).and_then(|()| out.flush());
}
