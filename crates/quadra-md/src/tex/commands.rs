//! The commands of the subset (behaviour/21, "Math"), delimiters and environments.

use super::alphabets::{ACCENTS, DELIMITERS, FONTS, Font, SPACES, slanted};
use super::lex::Tok;
use super::parse::{Parser, Until};
use super::symbols::{BIG_OPS, FUNCTIONS, SYMBOLS, find};
use super::{Class, Limits, Node};

/// The environments: rows of cells between `\begin` and `\end`.
pub const ENVIRONMENTS: [&str; 14] = [
    "matrix",
    "pmatrix",
    "bmatrix",
    "Bmatrix",
    "vmatrix",
    "Vmatrix",
    "smallmatrix",
    "cases",
    "aligned",
    "align",
    "align*",
    "gathered",
    "array",
    "split",
];

const STYLES: &[(&str, u8)] =
    &[("displaystyle", 0), ("textstyle", 1), ("scriptstyle", 2), ("scriptscriptstyle", 3)];
const BIGS: &[(&str, u8)] = &[("big", 1), ("Big", 2), ("bigg", 3), ("Bigg", 4)];

fn atom(text: &str, class: Class) -> Node {
    Node::Atom { text: text.to_string(), class }
}

impl<'a> Parser<'a> {
    /// The node of a command; none for one that says nothing by itself.
    pub(super) fn command(&mut self, name: &'a str) -> Option<Node> {
        if let Some((_, text, class)) = SYMBOLS.iter().find(|row| row.0 == name) {
            let mut letters = text.chars();
            return Some(match (letters.next(), letters.next()) {
                (Some(c @ '\u{3B1}'..='\u{3C9}'), None) if self.font == Font::Normal => {
                    atom(slanted(c).encode_utf8(&mut [0; 4]), *class)
                }
                _ => atom(text, *class),
            });
        }
        if let Some((_, glyph, limits)) = BIG_OPS.iter().find(|row| row.0 == name) {
            let limits = self.limits(if *limits { Limits::Display } else { Limits::Side });
            return Some(Node::BigOp { glyph, limits });
        }
        if let Some(limits) = find(FUNCTIONS, name) {
            let limits = self.limits(if limits { Limits::Display } else { Limits::Side });
            return Some(Node::Function { name: name.to_string(), limits });
        }
        if let Some(mark) = find(ACCENTS, name) {
            let body = self.arg();
            let line = mark.is_empty(); // `\overline`, `\underline`
            return Some(if line {
                Node::Line { body, under: name == "underline" }
            } else {
                Node::Accent { mark, body }
            });
        }
        if let Some(font) = find(FONTS, name) {
            let saved = std::mem::replace(&mut self.font, font);
            let body = self.arg();
            self.font = saved;
            return Some(Node::Group(body));
        }
        if let Some(amount) = find(SPACES, name) {
            return Some(Node::Space(amount));
        }
        if let Some(level) = find(STYLES, name) {
            return Some(Node::Style(level));
        }
        if let Some(level) = find(BIGS, name.trim_end_matches(['l', 'r', 'm'])) {
            let class = match name.as_bytes()[name.len() - 1] {
                b'l' => Class::Open,
                b'r' => Class::Close,
                _ => Class::Ord,
            };
            return Some(Node::Big { delim: self.delimiter(), level, class });
        }
        self.named(name)
    }

    fn named(&mut self, name: &str) -> Option<Node> {
        Some(match name {
            "frac" | "dfrac" | "tfrac" | "cfrac" => {
                let (over, under) = (self.arg(), self.arg());
                let style = match name {
                    "dfrac" | "cfrac" => Some(0),
                    "tfrac" => Some(1),
                    _ => None,
                };
                Node::Frac { over, under, rule: true, style }
            }
            "binom" | "dbinom" | "tbinom" => {
                let (over, under) = (self.arg(), self.arg());
                let body = vec![Node::Frac { over, under, rule: false, style: None }];
                Node::Delim { left: "(", right: ")", body }
            }
            "sqrt" => {
                let index = self.opt_arg().filter(|index| !index.is_empty());
                Node::Sqrt { index, body: self.arg() }
            }
            "left" => self.left_right(),
            "text" | "textrm" | "textit" | "textbf" | "mbox" | "textnormal" | "hbox" => {
                Node::Text(self.raw())
            }
            "operatorname" | "operatorname*" => {
                let name_of = self.raw();
                let limits = self.limits(if name.ends_with('*') { Limits::Display } else { Limits::Side });
                Node::Function { name: name_of, limits }
            }
            "overset" | "underset" | "stackrel" => {
                let (mark, body) = (self.arg(), self.arg());
                let class = if name == "stackrel" { Class::Rel } else { Class::Ord };
                let (over, under) = if name == "underset" { (None, Some(mark)) } else { (Some(mark), None) };
                Node::Stack { body, over, under, class }
            }
            "pmod" => {
                let mut body =
                    vec![Node::Space(6.0), atom("(", Class::Open), atom("mod", Class::Ord), Node::Space(6.0)];
                body.extend(self.arg());
                body.push(atom(")", Class::Close));
                Node::Group(body)
            }
            "not" => match self.atom()? {
                Node::Atom { text, class } => {
                    let struck = match text.as_str() {
                        "=" => "\u{2260}".to_string(),
                        "\u{2208}" => "\u{2209}".to_string(),
                        "\u{2282}" => "\u{2284}".to_string(),
                        "\u{2261}" => "\u{2262}".to_string(),
                        _ => text + "\u{338}",
                    };
                    Node::Atom { text: struck, class }
                }
                node => node,
            },
            "begin" => self.environment(),
            // Colours are not set: what is coloured shows in the colour of the text.
            "color" | "textcolor" => {
                self.raw();
                Node::Group(if name == "textcolor" { self.arg() } else { Vec::new() })
            }
            "limits" | "nolimits" | "mathstrut" | "strut" | "displaylimits" => return None,
            _ => {
                self.fail(format!("unknown command \\{name}"));
                Node::Text(format!("\\{name}"))
            }
        })
    }

    /// `\limits` or `\nolimits` after an operator; what it does by itself without them.
    fn limits(&mut self, by_itself: Limits) -> Limits {
        if self.take(Tok::Cmd("limits")) {
            Limits::Always
        } else if self.take(Tok::Cmd("nolimits")) {
            Limits::Side
        } else {
            by_itself
        }
    }

    /// The delimiter after `\left`, `\right` or `\big`.
    fn delimiter(&mut self) -> &'static str {
        let tok = self.next();
        let found = DELIMITERS.iter().find(|row| match tok {
            Tok::Cmd(name) => row.0.strip_prefix('\\') == Some(name),
            Tok::Char(c) => row.0.chars().eq([c]),
            _ => false,
        });
        if found.is_none() {
            let backslash = if matches!(tok, Tok::Cmd(_)) { "\\" } else { "" };
            self.fail(format!("bad delimiter {backslash}{}", tok.said()));
        }
        found.map_or("", |row| row.1)
    }

    fn left_right(&mut self) -> Node {
        let left = self.delimiter();
        let body = self.list(Until::End);
        let right = if self.next() == Tok::Cmd("right") {
            self.delimiter()
        } else {
            self.fail("\\left without \\right");
            ""
        };
        Node::Delim { left, right, body }
    }

    /// `\begin{env}` cells with `&` between them, rows with `\\`, `\end{env}`.
    fn environment(&mut self) -> Node {
        let said = self.raw();
        let env = ENVIRONMENTS.iter().find(|env| **env == said).copied();
        if env.is_none() {
            self.fail(format!("unknown environment {said}"));
        }
        if env == Some("array") {
            self.raw(); // how its columns are set: every one is centred
        }
        let (mut rows, mut row) = (Vec::new(), Vec::new());
        loop {
            row.push(self.list(Until::End));
            match self.next() {
                Tok::Amp => continue,
                Tok::Cmd("\\") => {
                    self.opt_arg(); // `\\[4pt]`
                    rows.push(std::mem::take(&mut row));
                    continue;
                }
                Tok::Cmd("end") => {
                    if self.raw() != said {
                        self.fail(format!("\\end does not match \\begin{{{said}}}"));
                    }
                }
                _ => self.fail(format!("missing \\end{{{said}}}")),
            }
            rows.push(row);
            break;
        }
        // A `\\` after the last row leaves a row with one empty cell.
        if rows.len() > 1 && rows.last().is_some_and(|row| row.len() == 1 && row[0].is_empty()) {
            rows.pop();
        }
        Node::Matrix { env: env.unwrap_or(""), rows }
    }
}
