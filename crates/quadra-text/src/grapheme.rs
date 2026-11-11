//! Grapheme cluster boundaries: what one caret step or one backspace covers.
//!
//! The rules cover combining marks, variation selectors, zero-width-joiner emoji sequences,
//! skin tones, regional-indicator flags and CRLF. Hangul and Indic conjuncts are on the
//! After-v1 list. The fake backend and the terminal tests fix this exact behaviour.

fn is_extend(c: char) -> bool {
    matches!(
        c as u32,
        0x0300..=0x036F
            | 0x1AB0..=0x1AFF
            | 0x1DC0..=0x1DFF
            | 0x20D0..=0x20FF
            | 0xFE20..=0xFE2F
            | 0xFE00..=0xFE0F
            | 0xE0100..=0xE01EF
            | 0x1F3FB..=0x1F3FF
            | 0x200C
    )
}

fn is_regional(c: char) -> bool {
    matches!(c as u32, 0x1F1E6..=0x1F1FF)
}

const JOINER: char = '\u{200D}';

/// The byte offset of the boundary after the cluster starting at `i`, which must be a
/// character boundary. Past the end it is the length.
pub fn next_grapheme(s: &str, i: usize) -> usize {
    let mut chars = s[i.min(s.len())..].char_indices();
    let Some((_, first)) = chars.next() else { return s.len() };
    if first == '\r' {
        return if matches!(chars.next(), Some((_, '\n'))) { i + 2 } else { i + 1 };
    }
    // Two regional indicators make a flag; a third starts the next one.
    let mut pair_open = is_regional(first);
    let mut prev = first;
    for (at, c) in chars {
        let joins = is_extend(c)
            || c == JOINER
            || prev == JOINER
            || (is_regional(c) && std::mem::take(&mut pair_open));
        if !joins {
            return i + at;
        }
        prev = c;
    }
    s.len()
}

/// The start of the cluster ending at byte offset `i`.
pub fn prev_grapheme(s: &str, i: usize) -> usize {
    let (mut last, mut at) = (0, 0);
    while at < i {
        last = at;
        at = next_grapheme(s, at);
    }
    last
}

/// Every grapheme boundary of `s`, from 0 to its length.
pub fn boundaries(s: &str) -> Vec<usize> {
    let mut out = vec![0];
    while *out.last().unwrap_or(&0) < s.len() {
        out.push(next_grapheme(s, out[out.len() - 1]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clusters_are_what_a_caret_step_covers() {
        assert_eq!(boundaries("abc"), [0, 1, 2, 3]);
        assert_eq!(boundaries(""), [0]);
        assert_eq!(boundaries("e\u{301}x"), [0, 3, 4]); // a combining mark stays with its base
        assert_eq!(boundaries("a\r\nb"), [0, 1, 3, 4]); // CRLF is one step
        assert_eq!(boundaries("\r\r\n"), [0, 1, 3]);
        assert_eq!(boundaries("\n\n"), [0, 1, 2]);
    }

    #[test]
    fn emoji_sequences_and_flags_are_single_clusters() {
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"; // joined with zero-width joiners
        assert_eq!(boundaries(family), [0, family.len()]);
        let thumbs = "\u{1F44D}\u{1F3FD}"; // a skin tone
        assert_eq!(boundaries(thumbs), [0, thumbs.len()]);
        let flags = "\u{1F1FB}\u{1F1F3}\u{1F1EF}\u{1F1F5}"; // two flags: pairs of regional indicators
        assert_eq!(boundaries(flags), [0, 8, 16]);
        let heart = "\u{2764}\u{FE0F}a"; // a variation selector
        assert_eq!(boundaries(heart), [0, 6, 7]);
    }

    #[test]
    fn stepping_back_finds_the_start_of_the_cluster() {
        let s = "a\u{1F1FB}\u{1F1F3}e\u{301}";
        assert_eq!(prev_grapheme(s, s.len()), 9);
        assert_eq!(prev_grapheme(s, 9), 1);
        assert_eq!((prev_grapheme(s, 1), prev_grapheme(s, 0)), (0, 0));
        assert_eq!(next_grapheme(s, 99), s.len());
    }
}
