//! How many cells a text takes where every cell is as wide as the next: the columns of
//! code, of a table written in text, of a terminal.

use crate::grapheme::next_grapheme;
use crate::width_table::{WIDE, ZERO};

fn within(c: u32, ranges: &[(u32, u32)]) -> bool {
    let at = ranges.partition_point(|range| range.1 < c);
    ranges.get(at).is_some_and(|range| range.0 <= c)
}

/// The cells of a character: none for a mark that joins the letter before it, two for an
/// East Asian wide character or an emoji, one for anything else.
pub fn char_width(c: char) -> usize {
    match c as u32 {
        c if c < 0x300 => 1,
        c if within(c, &ZERO) => 0,
        c => 1 + usize::from(within(c, &WIDE)),
    }
}

/// The cells of a grapheme: those of its widest character, so a letter with marks is one
/// and emoji joined into one are two.
pub fn grapheme_width(grapheme: &str) -> usize {
    grapheme.chars().map(char_width).max().unwrap_or(0)
}

/// The grapheme at `at`: where it ends, and its cells. ASCII that no mark follows is a
/// byte and a cell, without a look at any table.
pub fn cell_step(text: &str, at: usize) -> (usize, usize) {
    let bytes = text.as_bytes();
    if bytes[at] < 0x80 && bytes.get(at + 1).is_none_or(|next| *next < 0x80) {
        return (at + 1, 1);
    }
    let end = next_grapheme(text, at);
    (end, grapheme_width(&text[at..end]))
}

/// The cells of a text.
pub fn display_width(text: &str) -> usize {
    let (mut cells, mut at) = (0, 0);
    while at < text.len() {
        let (end, width) = cell_step(text, at);
        (cells, at) = (cells + width, end);
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::{char_width, display_width};

    #[test]
    fn a_text_takes_the_cells_of_its_graphemes() {
        assert_eq!(display_width("abc"), 3);
        // Letters with their marks in them, and with their marks after them.
        assert_eq!((display_width("Tiếng Việt"), display_width("Vie\u{302}\u{323}t")), (10, 4));
        assert_eq!(display_width("漢字"), 4);
        // People joined into one picture are one picture.
        assert_eq!(display_width("👩‍💻"), 2);
        assert_eq!((char_width('\u{301}'), char_width('😀'), char_width('A'), char_width('é')), (0, 2, 1, 1));
        assert_eq!(display_width(""), 0);
    }
}
