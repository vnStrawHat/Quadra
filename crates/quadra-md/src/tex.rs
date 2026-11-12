//! Math in the notation of TeX (specs/13, "Math"; the subset is behaviour/21): the parser,
//! plain functions over text. A formula comes from a document that is not ours: it costs
//! what its length costs, and nests no deeper than `DEEPEST`.

mod alphabets;
mod commands;
mod lex;
mod parse;
mod symbols;
#[cfg(test)]
mod tests;

pub use parse::{DEEPEST, parse};

/// What an atom is to its neighbours: it decides the space between them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Ord,
    Op,
    Bin,
    Rel,
    Open,
    Close,
    Punct,
    Inner,
}

/// Where the scripts of an operator go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limits {
    /// At its side: `\nolimits`, and what integrals and most functions do.
    Side,
    /// Above and below it in display, at its side in a line of text: `\sum`, `\lim`.
    Display,
    /// Above and below it: `\limits`.
    Always,
}

/// A list of nodes: what is between braces, an argument, a cell.
pub type List = Vec<Node>;

/// A node of a formula. A kind holds what it has and nothing else.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// Glyphs of one class.
    Atom {
        text: String,
        class: Class,
    },
    /// Words among the symbols, upright: `\text{..}`.
    Text(String),
    /// A list in braces: laid out as one ordinary atom.
    Group(List),
    /// One list over another, with a rule between them or without (`\binom`); the style is
    /// the one it is always set in (`\dfrac`: 0, `\tfrac`: 1).
    Frac {
        over: List,
        under: List,
        rule: bool,
        style: Option<u8>,
    },
    Sqrt {
        index: Option<List>,
        body: List,
    },
    /// A base with what is raised and lowered after it.
    Scripts {
        base: Box<Node>,
        sup: Option<List>,
        sub: Option<List>,
    },
    /// A big operator: `\sum`, `\int`.
    BigOp {
        glyph: &'static str,
        limits: Limits,
    },
    /// A name set upright: `\sin`, `\lim`, `\operatorname{..}`.
    Function {
        name: String,
        limits: Limits,
    },
    /// A list between delimiters that grow to it; an empty one is no delimiter.
    Delim {
        left: &'static str,
        right: &'static str,
        body: List,
    },
    /// A delimiter of a fixed size, 1 to 4: `\big` to `\Bigg`.
    Big {
        delim: &'static str,
        level: u8,
        class: Class,
    },
    /// A mark over a list: `\hat`, `\vec`.
    Accent {
        mark: &'static str,
        body: List,
    },
    /// A line over a list, or under it.
    Line {
        body: List,
        under: bool,
    },
    /// A space, in eighteenths of an em.
    Space(f32),
    /// Rows of cells; the environment says how they are set and between what.
    Matrix {
        env: &'static str,
        rows: Vec<Vec<List>>,
    },
    /// A list with one over it, one under it, or both: `\overset`, `\underset`.
    Stack {
        body: List,
        over: Option<List>,
        under: Option<List>,
        class: Class,
    },
    /// What follows it in its list is set in this style: 0 display, 1 text, 2 and 3 scripts.
    Style(u8),
}

impl Node {
    /// What the node is to its neighbours.
    pub fn class(&self) -> Class {
        match self {
            Node::Atom { class, .. } | Node::Big { class, .. } | Node::Stack { class, .. } => *class,
            Node::Scripts { base, .. } => base.class(),
            Node::BigOp { .. } | Node::Function { .. } => Class::Op,
            Node::Frac { .. } | Node::Delim { .. } | Node::Matrix { .. } => Class::Inner,
            _ => Class::Ord,
        }
    }
}
