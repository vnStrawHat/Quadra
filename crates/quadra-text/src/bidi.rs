//! Bidirectional text, for display (specs/08, "The DirectWrite backend"): which text needs
//! it, the direction of a text, and the visual order of what is at levels.

/// A character of a right-to-left script, or a mark that turns text right to left.
pub fn is_rtl(c: char) -> bool {
    matches!(
        c as u32,
        0x0590..=0x08FF
            | 0xFB1D..=0xFDFF
            | 0xFE70..=0xFEFF
            | 0x10800..=0x10FFF
            | 0x1E800..=0x1EFFF
            | 0x200F
            | 0x202B
            | 0x202E
            | 0x2067
    )
}

/// Whether a text has a right-to-left character: whether it needs bidirectional analysis.
pub fn has_rtl(text: &str) -> bool {
    !text.is_ascii() && text.chars().any(is_rtl)
}

/// Whether a text reads right to left: its first letter does (rule P2, with letters and the
/// two direction marks as the strong characters).
pub fn reads_rtl(text: &str) -> bool {
    let strong = |c: &char| c.is_alphabetic() || matches!(c, '\u{200E}' | '\u{200F}');
    text.chars().find(strong).is_some_and(is_rtl)
}

/// The visual order of items at bidirectional levels, left to right (rule L2): from the
/// highest level down to the lowest odd one, every run of items at that level or above is
/// reversed.
pub fn visual_order(levels: &[u8]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..levels.len()).collect();
    let (Some(&high), Some(&low)) = (levels.iter().max(), levels.iter().min()) else { return order };
    for level in ((low | 1)..=high).rev() {
        let mut i = 0;
        while i < order.len() {
            let start = i;
            while i < order.len() && levels[order[i]] >= level {
                i += 1;
            }
            order[start..i].reverse();
            i += 1;
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_reversed_from_the_highest_down() {
        assert_eq!(visual_order(&[0, 0, 0]), [0, 1, 2]);
        assert_eq!(visual_order(&[0, 1, 1, 0]), [0, 2, 1, 3]);
        // A number in right-to-left text keeps its order, and the text around it is turned.
        assert_eq!(visual_order(&[1, 1, 2, 2]), [2, 3, 1, 0]);
        assert_eq!(visual_order(&[1, 2, 1]), [2, 1, 0]);
    }

    #[test]
    fn a_text_reads_as_its_first_letter() {
        assert!(has_rtl("abc שלום") && !has_rtl("tiếng Việt"));
        assert!(reads_rtl("123 שלום abc") && !reads_rtl("abc שלום"));
    }
}
