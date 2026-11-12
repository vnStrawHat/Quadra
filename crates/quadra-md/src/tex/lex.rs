//! The tokens of a formula.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tok<'a> {
    End,
    /// `\name` without its backslash, or a backslash and one character that is no letter
    /// (`\,`, `\{`, `\\`). A star after a name is part of it: `\operatorname*`.
    Cmd(&'a str),
    Char(char),
    Open,
    Close,
    Sup,
    Sub,
    Amp,
    Space,
}

impl Tok<'_> {
    /// The token as it was written, a command without its backslash.
    pub fn said(self) -> String {
        match self {
            Tok::End => String::new(),
            Tok::Cmd(name) => name.to_string(),
            Tok::Char(c) => c.to_string(),
            Tok::Open => "{".into(),
            Tok::Close => "}".into(),
            Tok::Sup => "^".into(),
            Tok::Sub => "_".into(),
            Tok::Amp => "&".into(),
            Tok::Space => " ".into(),
        }
    }
}

/// The tokens of `source`, and an `End` after them.
pub fn lex(source: &str) -> Vec<Tok<'_>> {
    let mut out = Vec::new();
    let mut chars = source.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        let tok = match c {
            '\\' => match chars.next() {
                None => Tok::Char('\\'),
                Some((start, first)) => {
                    let mut end = start + first.len_utf8();
                    if first.is_ascii_alphabetic() {
                        while let Some((k, _)) = chars.next_if(|(_, c)| c.is_ascii_alphabetic()) {
                            end = k + 1;
                        }
                        end += usize::from(chars.next_if(|(_, c)| *c == '*').is_some());
                    }
                    Tok::Cmd(&source[start..end])
                }
            },
            '{' => Tok::Open,
            '}' => Tok::Close,
            '^' => Tok::Sup,
            '_' => Tok::Sub,
            '&' => Tok::Amp,
            '%' => {
                // A comment, to the end of its line.
                while chars.next_if(|(_, c)| *c != '\n').is_some() {}
                continue;
            }
            ' ' | '\t' | '\n' | '\r' => Tok::Space,
            c => Tok::Char(c),
        };
        out.push(tok);
    }
    out.push(Tok::End);
    out
}
