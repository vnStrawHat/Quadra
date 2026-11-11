//! IME composition through IMM32 (behaviour/02): the app draws the text being composed itself,
//! at its caret, and the OS's candidate window is placed beside that caret.

use std::ffi::c_void;

use quadra_platform::PlatformEvent;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::Input::Ime::{
    CANDIDATEFORM, CFS_EXCLUDE, CFS_POINT, COMPOSITIONFORM, GCS_COMPSTR, GCS_CURSORPOS, GCS_RESULTSTR, HIMC,
    IACE_DEFAULT, IME_COMPOSITION_STRING, ImmAssociateContextEx, ImmGetCompositionStringW, ImmGetContext,
    ImmReleaseContext, ImmSetCandidateWindow, ImmSetCompositionWindow,
};

/// The input context of a window, released when the call that took it is over.
struct Context(HWND, HIMC);

impl Context {
    fn of(hwnd: HWND) -> Option<Context> {
        // SAFETY: the window is alive; the context is released by `Drop`.
        let context = unsafe { ImmGetContext(hwnd) };
        (!context.is_invalid()).then_some(Context(hwnd, context))
    }

    /// One string of the composition, as UTF-16.
    fn read(&self, kind: IME_COMPOSITION_STRING) -> Vec<u16> {
        // SAFETY: the first call asks for the size in bytes; the buffer of the second holds
        // that many.
        unsafe {
            let bytes = ImmGetCompositionStringW(self.1, kind, None, 0).max(0) as usize;
            let mut units = vec![0u16; bytes / 2];
            if bytes > 0 {
                ImmGetCompositionStringW(
                    self.1,
                    kind,
                    Some(units.as_mut_ptr().cast::<c_void>()),
                    bytes as u32,
                );
            }
            units
        }
    }

    /// Puts the OS's composition and candidate windows at a caret, in device pixels of the
    /// window: the candidates open below it and never over it.
    fn place(&self, caret: RECT) {
        let composition = COMPOSITIONFORM {
            dwStyle: CFS_POINT,
            ptCurrentPos: POINT { x: caret.left, y: caret.top },
            rcArea: RECT::default(),
        };
        let candidates = CANDIDATEFORM {
            dwIndex: 0,
            dwStyle: CFS_EXCLUDE,
            ptCurrentPos: POINT { x: caret.left, y: caret.bottom },
            rcArea: caret,
        };
        // SAFETY: the structures are complete and valid for the calls.
        unsafe {
            let _ = ImmSetCompositionWindow(self.1, &composition);
            let _ = ImmSetCandidateWindow(self.1, &candidates);
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: the context was taken for this window by `Context::of`.
        let _ = unsafe { ImmReleaseContext(self.0, self.1) };
    }
}

/// The byte offset in the UTF-8 form of `units` of the UTF-16 offset `at`.
pub(crate) fn utf8_offset(units: &[u16], at: usize) -> usize {
    String::from_utf16_lossy(&units[..at.min(units.len())]).len()
}

/// What one `WM_IME_COMPOSITION` says: text that was committed, then the text still being
/// composed with its caret. `flags` is the message's `lparam`.
pub(crate) fn composition(hwnd: HWND, flags: u32, caret: RECT) -> Vec<PlatformEvent> {
    let Some(context) = Context::of(hwnd) else { return Vec::new() };
    let mut events = Vec::new();
    if flags & GCS_RESULTSTR.0 != 0 {
        let committed = context.read(GCS_RESULTSTR);
        if !committed.is_empty() {
            events.push(PlatformEvent::Text(String::from_utf16_lossy(&committed)));
        }
    }
    if flags & GCS_COMPSTR.0 != 0 {
        let composing = context.read(GCS_COMPSTR);
        // SAFETY: asking for the caret position needs no buffer: it is the return value.
        let at = unsafe { ImmGetCompositionStringW(context.1, GCS_CURSORPOS, None, 0) }.max(0) as usize;
        let caret = utf8_offset(&composing, at);
        events.push(PlatformEvent::Composition { text: String::from_utf16_lossy(&composing), caret });
    }
    context.place(caret);
    events
}

/// Moves the candidate window to a caret that moved.
pub(crate) fn place(hwnd: HWND, caret: RECT) {
    if let Some(context) = Context::of(hwnd) {
        context.place(caret);
    }
}

/// Turns the input method on for a window (text is expected) or off (keys are keys).
pub(crate) fn enable(hwnd: HWND, on: bool) {
    // SAFETY: the window is alive. Its default context is given back, or none is associated.
    let _ = unsafe { ImmAssociateContextEx(hwnd, HIMC::default(), if on { IACE_DEFAULT } else { 0 }) };
}

#[cfg(test)]
mod tests {
    use super::utf8_offset;

    #[test]
    fn a_caret_in_utf16_units_is_a_byte_offset_in_utf8() {
        let units: Vec<u16> = "việt 😀!".encode_utf16().collect();
        // "v", "i" are one byte each; "ệ" is three; the emoji is two units and four bytes.
        let offsets: Vec<usize> = [0, 2, 3, 5, 7, 8, 99].iter().map(|&at| utf8_offset(&units, at)).collect();
        assert_eq!(offsets, [0, 2, 5, 7, 11, 12, 12]);
    }
}
