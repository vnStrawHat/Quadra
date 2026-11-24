//! The status bar (specs/21, "Window"): the document's words, characters and reading time;
//! the editor's cursor while it shows; the lines and the line ends.

use quadra::{View, memo, row, show};
use quadra_design::{status_divider, status_item, statusbar};

use crate::model::{Model, PREVIEW};
use quadra_design::token;

/// Words a minute, read.
const READ: usize = 200;

/// The words, the characters and the lines of a text: a word is a run without spaces that
/// holds a letter or a digit; characters leave out line ends.
pub fn counts(text: &str) -> (usize, usize, usize) {
    let words = text.split_whitespace().filter(|it| it.chars().any(char::is_alphanumeric)).count();
    let chars = text.chars().filter(|c| *c != '\n' && *c != '\r').count();
    (words, chars, text.lines().count().max(1))
}

/// `1234` as `1,234`.
fn grouped(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (k, c) in digits.chars().enumerate() {
        if k > 0 && (digits.len() - k).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn some(n: usize, one: &str) -> String {
    format!("{} {one}{}", grouped(n), if n == 1 { "" } else { "s" })
}

pub(crate) fn bar(m: &Model) -> impl View + use<> {
    let text = m.text;
    let counted = memo(move || text.with(|text| counts(text)));
    let read = move || {
        let (words, chars, _) = counted.get();
        let minutes = words.div_ceil(READ).max(1);
        format!("{} · {} · {minutes} min read", some(words, "word"), some(chars, "character"))
    };
    let (layout, place) = (m.layout, m.place);
    let caret = move || {
        let at = place.get();
        let selected =
            if at.selected > 0 { format!(" ({} selected)", grouped(at.selected)) } else { String::new() };
        format!("Ln {}, Col {}{selected}", at.line, at.column)
    };
    // The editor shows: its cursor.
    let editing = show(
        move || layout.get() != PREVIEW,
        move || row().gap(token::SPACE_LG).child(status_item(None, caret)).child(status_divider()),
    );
    let saved = m.saved;
    let crlf = memo(move || saved.with(|it| it.contains("\r\n")));
    statusbar()
        .left(status_item(None, read))
        .right(editing)
        .right(status_item(None, move || some(counted.get().2, "line")))
        .right(status_item(None, move || if crlf.get() { "CRLF" } else { "LF" }.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_are_no_words_and_line_ends_no_characters() {
        assert_eq!(counts("# Notes\n\n- one two\n| a | b |\r\n"), (5, 25, 4));
        assert_eq!(grouped(1234567), "1,234,567");
        assert_eq!(some(1, "word"), "1 word");
    }
}
