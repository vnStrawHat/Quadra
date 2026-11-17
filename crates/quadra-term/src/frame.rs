//! The frames a program closes (specs/18, "Delivery"): a read is fed in parts that end where a
//! synchronized update ends (`CSI ? 2026 l`), so the reader thread can copy each frame then.
//! A program that draws without a pause opens the next update in the same read, and the UI
//! thread, which takes its snapshot later, would find every frame held.

const CLOSE: &[u8] = b"\x1b[?2026l";

/// Where in `bytes` the next `CSI ? 2026 l` ends (just past it), given how much of it the end
/// of the last read matched (`matched`, kept for the next read).
pub(crate) fn close_end(matched: &mut usize, bytes: &[u8]) -> Option<usize> {
    for (i, &b) in bytes.iter().enumerate() {
        // No proper prefix of CLOSE is also its suffix: a mismatch restarts at ESC or nothing.
        *matched = if b == CLOSE[*matched] { *matched + 1 } else { usize::from(b == CLOSE[0]) };
        if *matched == CLOSE.len() {
            *matched = 0;
            return Some(i + 1);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_close_is_found_whole_or_across_two_reads() {
        let mut m = 0;
        assert_eq!(close_end(&mut m, b"ab\x1b[?2026lcd"), Some(10));
        assert_eq!(close_end(&mut m, b"x\x1b[?20"), None);
        assert_eq!(m, 5);
        assert_eq!(close_end(&mut m, b"26lz"), Some(3));
        assert_eq!(close_end(&mut m, b"\x1b\x1b[?2026h\x1b[?2026l"), Some(17));
        assert_eq!(close_end(&mut m, b"\x1b[?2026h"), None);
        assert_eq!(m, 0);
    }
}
