//! The parser: tokens to the nodes of a formula. A fault is noted, the first one is what is
//! told, and the parser goes on: a formula with a slip in it still shows what it can.

use super::alphabets::{Font, styled};
use super::lex::{Tok, lex};
use super::{Class, List, Node};

/// How deep lists and atoms nest at most (a brace is two): deeper is a fault of the
/// formula, for the parser and for whatever walks its nodes after it.
pub const DEEPEST: usize = 256;

/// What ends a list, besides what ends every list.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Until {
    End,
    Brace,
    Bracket,
}

pub(super) struct Parser<'a> {
    toks: Vec<Tok<'a>>,
    pos: usize,
    pub font: Font,
    error: Option<String>,
    depth: usize,
}

/// The nodes of a formula (its source without the `$` around it), and the first thing
/// that is wrong with it, if anything is.
pub fn parse(source: &str) -> (List, Option<String>) {
    let mut p = Parser { toks: lex(source), pos: 0, font: Font::Normal, error: None, depth: 0 };
    let nodes = p.list(Until::End);
    if p.error.is_none() && p.peek() != Tok::End {
        let said = p.peek().said();
        p.fail(format!("unexpected {said}"));
    }
    (nodes, p.error)
}

/// The prime in a superscript for one, two, and three or more marks.
const PRIMES: [&str; 3] = ["\u{2032}", "\u{2033}", "\u{2034}"];

impl<'a> Parser<'a> {
    pub fn fail(&mut self, message: impl Into<String>) {
        self.error.get_or_insert_with(|| message.into());
    }

    /// One level deeper; false past the deepest, where the formula is at fault and what
    /// is left of it is skipped, so that every caller returns at once.
    fn enter(&mut self) -> bool {
        self.depth += 1;
        if self.depth > DEEPEST {
            self.fail("formula nested too deeply");
            self.pos = self.toks.len() - 1;
        }
        self.depth <= DEEPEST
    }

    /// The next token that is no space, left where it is.
    pub fn peek(&mut self) -> Tok<'a> {
        while self.toks[self.pos] == Tok::Space {
            self.pos += 1;
        }
        self.toks[self.pos]
    }

    pub fn next(&mut self) -> Tok<'a> {
        let tok = self.peek();
        self.pos += usize::from(tok != Tok::End);
        tok
    }

    /// Goes past the next token when it is `tok`.
    pub fn take(&mut self, tok: Tok) -> bool {
        let found = self.peek() == tok;
        self.pos += usize::from(found);
        found
    }

    fn stops(tok: Tok, until: Until) -> bool {
        match tok {
            Tok::End | Tok::Amp => true,
            Tok::Close => until == Until::Brace,
            Tok::Char(']') => until == Until::Bracket,
            Tok::Cmd(name) => matches!(name, "\\" | "right" | "end" | "middle"),
            _ => false,
        }
    }

    /// Atoms with their scripts, up to a token that ends the list, which is left there.
    pub fn list(&mut self, until: Until) -> List {
        let mut out = Vec::new();
        if self.enter() {
            loop {
                let tok = self.peek();
                if Self::stops(tok, until) {
                    break;
                }
                if matches!(tok, Tok::Sup | Tok::Sub | Tok::Char('\'')) {
                    self.scripts(&mut out);
                } else {
                    out.extend(self.atom());
                }
            }
        }
        self.depth -= 1;
        out
    }

    /// `^`, `_` or primes: they belong to the node before them.
    fn scripts(&mut self, out: &mut List) {
        let tok = self.next();
        let (base, mut sup, mut sub) = match out.pop() {
            Some(Node::Scripts { base, sup, sub }) => (base, sup, sub),
            Some(node) => (Box::new(node), None, None),
            None => (Box::new(Node::Group(Vec::new())), None, None),
        };
        let prime = |text: &str| Node::Atom { text: text.to_string(), class: Class::Ord };
        match tok {
            Tok::Sup => {
                // What is raised goes after primes, in the one superscript.
                let primed = matches!(sup.as_deref(), Some([Node::Atom { text, .. }]) if PRIMES.contains(&text.as_str()));
                if sup.is_some() && !primed {
                    self.fail("double superscript");
                }
                let mut list = sup.take().filter(|_| primed).unwrap_or_default();
                list.extend(self.arg());
                sup = Some(list);
            }
            Tok::Sub => {
                if sub.is_some() {
                    self.fail("double subscript");
                }
                sub = Some(self.arg());
            }
            _ => {
                let mut count = 1;
                while self.toks[self.pos] == Tok::Char('\'') {
                    self.pos += 1;
                    count += 1;
                }
                sup = Some(vec![prime(PRIMES[count.min(3) - 1])]);
            }
        }
        out.push(Node::Scripts { base, sup, sub });
    }

    /// A list in braces, or one atom.
    pub fn arg(&mut self) -> List {
        let tok = self.peek();
        if tok == Tok::Open {
            self.pos += 1;
            return self.braced();
        }
        if Self::stops(tok, Until::End) || matches!(tok, Tok::Sup | Tok::Sub) {
            self.fail("missing argument");
            return Vec::new();
        }
        self.atom().into_iter().collect()
    }

    /// The list after a `{`, and past its `}`.
    fn braced(&mut self) -> List {
        let body = self.list(Until::Brace);
        if !self.take(Tok::Close) {
            self.fail("missing }");
        }
        body
    }

    /// An argument in brackets, when there is one: `\sqrt[3]{x}`.
    pub fn opt_arg(&mut self) -> Option<List> {
        if !self.take(Tok::Char('[')) {
            return None;
        }
        let body = self.list(Until::Bracket);
        if !self.take(Tok::Char(']')) {
            self.fail("missing ]");
        }
        Some(body)
    }

    /// The text of an argument in braces, its spaces kept: `\text`, `\begin`.
    pub fn raw(&mut self) -> String {
        if self.peek() != Tok::Open {
            return self.next().said();
        }
        self.pos += 1;
        let (mut depth, mut text) = (1, String::new());
        while self.toks[self.pos] != Tok::End {
            let tok = self.toks[self.pos];
            self.pos += 1;
            match tok {
                Tok::Open => depth += 1,
                Tok::Close => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                return text;
            }
            match tok {
                // A space by its command is a space, and a character by its is itself.
                Tok::Cmd("," | ":" | ";" | ">" | " " | "quad" | "qquad") => text.push(' '),
                Tok::Cmd("!") => {}
                Tok::Cmd(name) if name.chars().count() > 1 => text.extend(["\\", name]),
                tok => text.push_str(&tok.said()),
            }
        }
        self.fail("missing }");
        text
    }

    pub fn atom(&mut self) -> Option<Node> {
        let node = if self.enter() {
            match self.next() {
                Tok::Open => Some(Node::Group(self.braced())),
                Tok::Char(c) => Some(self.char_atom(c)),
                Tok::Cmd(name) => self.command(name),
                tok => {
                    self.fail(format!("unexpected {}", tok.said()));
                    None
                }
            }
        } else {
            None
        };
        self.depth -= 1;
        node
    }

    fn char_atom(&self, c: char) -> Node {
        let (text, class) = match c {
            // Digits are upright where letters are slanted.
            '0'..='9' if self.font == Font::Normal => (c, Class::Ord),
            'a'..='z' | 'A'..='Z' | '0'..='9' | '\u{3B1}'..='\u{3C9}' => (styled(c, self.font), Class::Ord),
            '+' => ('+', Class::Bin),
            '-' => ('\u{2212}', Class::Bin),
            '*' => ('\u{2217}', Class::Bin),
            '=' | '<' | '>' | ':' => (c, Class::Rel),
            '(' | '[' => (c, Class::Open),
            ')' | ']' | '!' | '?' => (c, Class::Close),
            ',' | ';' => (c, Class::Punct),
            '~' => (' ', Class::Ord),
            _ => (c, Class::Ord),
        };
        Node::Atom { text: text.to_string(), class }
    }
}
