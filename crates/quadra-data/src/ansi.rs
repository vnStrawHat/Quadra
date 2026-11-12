//! The escape sequences a line of a log colours itself with (specs/13, "Data"): SGR, the
//! part of what terminals know that says how text looks. Every other sequence is dropped.

use std::borrow::Cow;

/// What a run of text is written in.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Ink {
    /// The colour of text.
    #[default]
    Text,
    /// One of the sixteen colours of the theme.
    Palette(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pen {
    pub ink: Ink,
    pub bold: bool,
    pub faint: bool,
}

/// One of the 256 colours: the sixteen, a cube of six levels a channel, and greys.
fn indexed(n: u8) -> Ink {
    let level = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
    match n {
        0..=15 => Ink::Palette(n),
        16..=231 => Ink::Rgb(level((n - 16) / 36), level((n - 16) / 6 % 6), level((n - 16) % 6)),
        _ => Ink::Rgb(8 + 10 * (n - 232), 8 + 10 * (n - 232), 8 + 10 * (n - 232)),
    }
}

impl Pen {
    /// Takes the parameters of one SGR sequence.
    fn set(&mut self, params: &str) {
        let mut codes = params.split(';').map(|code| code.parse::<u16>().unwrap_or(0));
        while let Some(code) = codes.next() {
            match code {
                0 => *self = Pen::default(),
                1 => self.bold = true,
                2 => self.faint = true,
                22 => (self.bold, self.faint) = (false, false),
                30..=37 => self.ink = Ink::Palette((code - 30) as u8),
                39 => self.ink = Ink::Text,
                90..=97 => self.ink = Ink::Palette((code - 90 + 8) as u8),
                // A colour by its number or its channels; one for the background is read
                // and not kept.
                38 | 48 => {
                    let mut byte = || codes.next().unwrap_or(0).min(255) as u8;
                    let ink = match byte() {
                        5 => indexed(byte()),
                        2 => Ink::Rgb(byte(), byte(), byte()),
                        _ => continue,
                    };
                    if code == 38 {
                        self.ink = ink;
                    }
                }
                _ => {}
            }
        }
    }
}

/// Ends a run at `end`, if anything was written since the one before.
fn close(runs: &mut Vec<(usize, Pen)>, end: usize, pen: Pen) {
    if end > runs.last().map_or(0, |(before, _)| *before) {
        runs.push((end, pen));
    }
}

/// What a line says, and the pens it is written in: each with where its run ends.
pub fn runs(line: &str) -> (String, Vec<(usize, Pen)>) {
    let (mut said, mut runs, mut pen) = (String::with_capacity(line.len()), Vec::new(), Pen::default());
    let mut rest = line;
    while let Some(at) = rest.find(['\u{1b}', '\r']) {
        said.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        // ESC [ parameters and a letter: a sequence. Anything else: the one character goes.
        let Some(body) = after.strip_prefix('[').filter(|_| rest.as_bytes()[at] == 0x1b) else {
            rest = after;
            continue;
        };
        // A sequence that does not end takes the rest of the line with it.
        let end = body.find(|c: char| ('\u{40}'..='\u{7e}').contains(&c)).unwrap_or(body.len());
        if body[end..].starts_with('m') {
            close(&mut runs, said.len(), pen);
            pen.set(&body[..end]);
        }
        rest = body.get(end + 1..).unwrap_or("");
    }
    said.push_str(rest);
    close(&mut runs, said.len(), pen);
    (said, runs)
}

/// What a line says, without its escape sequences.
pub fn plain(line: &str) -> Cow<'_, str> {
    if line.contains(['\u{1b}', '\r']) { Cow::Owned(runs(line).0) } else { Cow::Borrowed(line) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_what_it_says_and_the_pens_it_is_written_in() {
        let (said, pens) = runs("\u{1b}[31mERROR\u{1b}[0m request \u{1b}[1;92m7\u{1b}[2m ok");
        assert_eq!(said, "ERROR request 7 ok");
        let red = Pen { ink: Ink::Palette(1), ..Pen::default() };
        let green = Pen { ink: Ink::Palette(10), bold: true, faint: false };
        assert_eq!(pens, [(5, red), (14, Pen::default()), (15, green), (18, Pen { faint: true, ..green })]);
        // By number and by channels; a background is read and not kept; what is not SGR goes.
        let (said, pens) = runs("\u{1b}[38;5;196ma\u{1b}[48;2;1;2;3;38;2;10;20;30mb\u{1b}[2K\u{1b}[39mc\r");
        assert_eq!((said.as_str(), pens.len()), ("abc", 3));
        assert_eq!(
            (pens[0].1.ink, pens[1].1.ink, pens[2].1.ink),
            (Ink::Rgb(255, 0, 0), Ink::Rgb(10, 20, 30), Ink::Text)
        );
        assert_eq!((indexed(244), indexed(21)), (Ink::Rgb(128, 128, 128), Ink::Rgb(0, 0, 255)));
    }

    #[test]
    fn a_line_without_sequences_is_itself_and_a_broken_one_ends() {
        assert!(matches!(plain("INFO é ok"), Cow::Borrowed("INFO é ok")));
        assert_eq!(runs("plain"), ("plain".to_string(), vec![(5, Pen::default())]));
        assert_eq!(plain("\u{1b}[1;32mok\u{1b}[0m!"), "ok!");
        // A sequence that never ends, an escape at the end, an empty line.
        assert_eq!(
            (plain("a\u{1b}[31"), plain("b\u{1b}"), plain("é\u{1b}é")),
            ("a".into(), "b".into(), "éé".into())
        );
        assert_eq!(runs(""), (String::new(), Vec::new()));
    }
}
