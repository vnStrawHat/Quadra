//! The tokenizer of Markdown. States: 0, 1 in a ``` fence, 2 in a ~~~ fence, 3 in `$$`
//! math. Headings are tags, strong text numbers, emphasis keywords, code strings, the text
//! of a link a function and its address a comment, math escapes, markers operators.

use crate::lex::{Lex, is_ident};
use crate::token::{Token, TokenKind};

pub fn tokenize(line: &str, state: u32, out: &mut Vec<Token>) -> u32 {
    let mut l = Lex::new(line, out);
    l.skip_space();
    let rest = &line.as_bytes()[l.i..];
    let fence = match rest {
        [b'`', b'`', b'`', ..] => 1,
        [b'~', b'~', b'~', ..] => 2,
        _ => 0,
    };
    let whole = |l: &mut Lex, kind| {
        l.i = line.len();
        l.add(0, kind);
    };
    if state == 1 || state == 2 {
        let closes = fence == state;
        whole(&mut l, if closes { TokenKind::Punctuation } else { TokenKind::Str });
        return if closes { 0 } else { state };
    }
    if state == 3 || rest.starts_with(b"$$") {
        whole(&mut l, TokenKind::Escape);
        let closes = rest.ends_with(b"$$") && (state == 3 || rest.len() > 2);
        return if closes { 0 } else { 3 };
    }
    if fence > 0 {
        l.i += 3;
        l.add(l.i - 3, TokenKind::Punctuation);
        let a = l.i;
        l.i = line.len();
        l.add(a, TokenKind::Type);
        return fence;
    }
    let mut marks = rest.iter().filter(|c| **c != b' ');
    let rule = marks.clone().count() >= 3 && marks.all(|c| matches!(c, b'-' | b'*' | b'_'));
    if rule || rest.starts_with(b"#") {
        whole(&mut l, if rule { TokenKind::Punctuation } else { TokenKind::Tag });
        return 0;
    }
    while l.at(0) == b'>' {
        l.i += 1;
        l.add(l.i - 1, TokenKind::Comment);
        l.skip_space();
    }
    l.list_marker();
    l.md_inline();
    0
}

/// Where a mark is next. A line is not searched twice for the same mark: a search that
/// starts later finds the same place while that place is still ahead, and nothing once
/// nothing was found. This keeps a long line of marks that never close linear.
#[derive(Clone, Copy, Default)]
struct Seek {
    searched: bool,
    from: usize,
    found: Option<usize>,
}

impl Seek {
    fn find(&mut self, s: &[u8], from: usize, mark: &[u8]) -> Option<usize> {
        let known = self.searched && from >= self.from && self.found.is_none_or(|at| at >= from);
        if !known {
            let at = s.get(from..).and_then(|rest| rest.windows(mark.len()).position(|w| w == mark));
            *self = Seek { searched: true, from, found: at.map(|at| at + from) };
        }
        self.found
    }
}

/// The marks a line is searched for, each with its own `Seek`.
const MARKS: usize = 8;
const TICK: usize = 0;
const DOLLAR: usize = 1;
const STRONG: usize = 2;
const STRONG_LOW: usize = 3;
const STAR: usize = 4;
const LOW: usize = 5;
const BRACKET: usize = 6;
const PAREN: usize = 7;

impl Lex<'_> {
    /// `- `, `1. ` and the box of a task: `[ ] `.
    fn list_marker(&mut self) {
        let a = self.i;
        if matches!(self.at(0), b'-' | b'*' | b'+') && self.at(1) == b' ' {
            self.i += 1;
        } else {
            self.skip_while(|c| c.is_ascii_digit());
            if self.i == a || !matches!(self.at(0), b'.' | b')') || self.at(1) != b' ' {
                self.i = a;
                return;
            }
            self.i += 1;
        }
        self.add(a, TokenKind::Operator);
        self.skip_space();
        if self.at(0) == b'[' && self.at(2) == b']' && matches!(self.at(1), b' ' | b'x' | b'X') {
            self.i += 3;
            self.add(self.i - 3, TokenKind::Operator);
        }
    }

    /// The marks in the rest of the line.
    fn md_inline(&mut self) {
        let mut seeks = [Seek::default(); MARKS];
        let mut no_url_to = 0;
        while !self.done() {
            let (c, a, n) = (self.s[self.i], self.i, self.at(1));
            let opens = n != b' ' && n != 0;
            match c {
                b'\\' => self.escape(),
                b'`' => self.span_to(&mut seeks[TICK], a, b"`", TokenKind::Str),
                b'$' if opens => self.span_to(&mut seeks[DOLLAR], a, b"$", TokenKind::Escape),
                b'*' if n == c => self.span_to(&mut seeks[STRONG], a + 1, b"**", TokenKind::Number),
                b'_' if n == c => self.span_to(&mut seeks[STRONG_LOW], a + 1, b"__", TokenKind::Number),
                b'*' if opens => self.span_to(&mut seeks[STAR], a, b"*", TokenKind::Keyword),
                b'_' if opens && !is_ident(self.before(a)) => {
                    self.span_to(&mut seeks[LOW], a, b"_", TokenKind::Keyword);
                }
                b'[' | b'!' if self.link(&mut seeks, a) => {}
                b'<' | b'h' if self.url(a, &mut no_url_to) => {}
                _ => self.i += 1,
            }
        }
    }

    /// From where it is through the mark that closes, when the line has one after `from`
    /// and the mark there: a token of the kind. Else past the mark that opens only.
    fn span_to(&mut self, seek: &mut Seek, from: usize, mark: &[u8], kind: TokenKind) {
        let a = self.i;
        match seek.find(self.s, from + mark.len(), mark) {
            Some(end) => {
                self.i = end + mark.len();
                self.add(a, kind);
            }
            None => self.i = a + mark.len(),
        }
    }

    /// `[text](address)` and `![text](address)`; false when none is here.
    fn link(&mut self, seeks: &mut [Seek; MARKS], a: usize) -> bool {
        if self.s[a] == b'!' && self.at(1) != b'[' {
            return false;
        }
        let Some(close) = seeks[BRACKET].find(self.s, a + 1, b"]") else { return false };
        if self.byte(close + 1) != b'(' {
            return false;
        }
        let Some(end) = seeks[PAREN].find(self.s, close + 2, b")") else { return false };
        self.i = close + 1;
        self.add(a, TokenKind::Function);
        self.i = end + 1;
        self.add(close + 1, TokenKind::Comment);
        true
    }

    /// `<scheme://..>` and a bare `http://..`, to a space or a `>`; false when none is
    /// here. `clean_to` is how far the line is known to have no address in angle brackets.
    fn url(&mut self, a: usize, clean_to: &mut usize) -> bool {
        let bare = self.starts_at("http://", false) || self.starts_at("https://", false);
        if !(bare || (self.s[a] == b'<' && a >= *clean_to)) {
            return false;
        }
        let e = a + 1 + self.s[a + 1..].iter().take_while(|c| !matches!(c, b' ' | b'>')).count();
        if !self.s[a..e].windows(3).any(|w| w == b"://") {
            *clean_to = e;
            return false;
        }
        self.i = e + usize::from(self.byte(e) == b'>');
        self.add(a, TokenKind::Comment);
        true
    }
}
