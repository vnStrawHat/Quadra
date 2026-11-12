//! Flowcharts as they are written: nodes with shapes, links with strokes, heads and labels,
//! chains, groups with `&`, and subgraphs. Lines that style (classDef, style, click,
//! linkStyle) say nothing here.

use std::collections::HashMap;

use super::draw::{Head, decode};
use super::scan::Scan;

pub(super) mod draw;
pub(super) mod order;
pub(super) mod place;
pub(super) mod rank;

/// What a diagram may hold: one past these is a fault, shown as its source, so a document
/// cannot make the layout work or allocate without bound.
pub const MOST_NODES: usize = 1000;
/// `a & b --> c & d` links every pair.
pub const MOST_EDGES: usize = 500;
pub const MOST_SUBGRAPHS: usize = 100;
/// The nodes long links are cut into, one a rank they cross.
pub const MOST_DUMMIES: usize = 5000;
/// The ranks one link may ask for: `a ------> b`.
pub const LONGEST_LINK: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// `id[text]`
    Rect,
    /// `id(text)`
    Round,
    /// `id([text])`
    Stadium,
    /// `id((text))`
    Circle,
    /// `id{text}`
    Diamond,
    /// `id{{text}}`
    Hexagon,
    /// `id[[text]]`
    Subroutine,
    /// `id[(text)]`
    Cylinder,
    /// `id>text]`
    Flag,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stroke {
    Solid,
    Dotted,
    Thick,
}

/// Which way the ranks go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Down,
    Up,
    Right,
    Left,
}

impl Dir {
    /// Ranks are rows: the chart goes down or up.
    pub fn vertical(self) -> bool {
        matches!(self, Dir::Down | Dir::Up)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FNode {
    pub id: String,
    pub label: String,
    pub shape: Shape,
    /// The innermost subgraph it is in.
    pub sub: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FEdge {
    pub from: usize,
    pub to: usize,
    pub label: String,
    pub stroke: Stroke,
    /// At `to`.
    pub head: Head,
    /// At `from`: `<-->`.
    pub tail: Head,
    /// How many ranks it spans at least.
    pub span: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sub {
    pub title: String,
    pub parent: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Flow {
    pub dir: Option<Dir>,
    pub nodes: Vec<FNode>,
    pub edges: Vec<FEdge>,
    pub subs: Vec<Sub>,
    ids: HashMap<String, usize>,
}

const IGNORED: [&str; 6] = ["classDef", "class", "style", "click", "linkStyle", "direction"];

/// What opens the text of a node, what closes it, and the shape that is.
const OPENERS: [(&str, &str, Shape); 9] = [
    ("[[", "]]", Shape::Subroutine),
    ("[(", ")]", Shape::Cylinder),
    ("((", "))", Shape::Circle),
    ("([", "])", Shape::Stadium),
    ("{{", "}}", Shape::Hexagon),
    ("[", "]", Shape::Rect),
    ("(", ")", Shape::Round),
    ("{", "}", Shape::Diamond),
    (">", "]", Shape::Flag),
];

struct Link {
    label: String,
    stroke: Stroke,
    head: Head,
    tail: Head,
    span: usize,
}

/// The flowchart of the lines after its header; `first` is the number of the header's
/// line, for what a fault says.
pub fn parse(dir: &str, lines: &[&str], first: usize) -> Result<Flow, String> {
    let dir = match dir {
        "BT" => Dir::Up,
        "LR" => Dir::Right,
        "RL" => Dir::Left,
        _ => Dir::Down,
    };
    let mut flow = Flow { dir: Some(dir), ..Flow::default() };
    let mut open: Vec<usize> = Vec::new();
    for (n, raw) in lines.iter().enumerate() {
        for statement in raw.split(';') {
            let t = statement.trim();
            let word = t.split(' ').next().unwrap_or("");
            if t.is_empty() || t.starts_with("%%") || IGNORED.contains(&word) {
                continue;
            }
            if word == "subgraph" {
                if flow.subs.len() >= MOST_SUBGRAPHS {
                    return Err(format!("more than {MOST_SUBGRAPHS} subgraphs"));
                }
                let (rest, parent) = (t["subgraph".len()..].trim(), open.last().copied());
                let title = match (rest.find('['), rest.strip_suffix(']')) {
                    (Some(at), Some(inside)) => decode(&inside[at + 1..]),
                    _ => decode(rest),
                };
                open.push(flow.subs.len());
                flow.subs.push(Sub { title, parent });
            } else if t == "end" {
                open.pop();
            } else {
                let fault = |fault: String| format!("line {}: {fault}", first + n + 1);
                flow.statement(&mut Scan::new(t), open.last().copied()).map_err(fault)?;
            }
        }
    }
    let dummies = rank::dummies(&flow);
    if dummies > MOST_DUMMIES { Err("the diagram is too large to lay out".to_string()) } else { Ok(flow) }
}

impl Flow {
    pub fn dir(&self) -> Dir {
        self.dir.unwrap_or(Dir::Down)
    }

    /// A group, then links each with the group after it; a group is nodes with `&`
    /// between them.
    fn statement(&mut self, sc: &mut Scan, sub: Option<usize>) -> Result<(), String> {
        let mut from = self.group(sc, sub)?;
        loop {
            sc.space();
            if sc.done() {
                return Ok(());
            }
            let link = link(sc).ok_or_else(|| format!("expected a link at `{}`", sc.rest_shown()))?;
            let to = self.group(sc, sub)?;
            if self.edges.len() + from.len() * to.len() > MOST_EDGES {
                return Err(format!("more than {MOST_EDGES} links"));
            }
            for (a, b) in from.iter().flat_map(|a| to.iter().map(move |b| (*a, *b))) {
                let Link { label, stroke, head, tail, span } = &link;
                let (label, stroke, head, tail, span) = (label.clone(), *stroke, *head, *tail, *span);
                self.edges.push(FEdge { from: a, to: b, label, stroke, head, tail, span });
            }
            from = to;
        }
    }

    fn group(&mut self, sc: &mut Scan, sub: Option<usize>) -> Result<Vec<usize>, String> {
        let mut out = vec![self.node(sc, sub)?];
        loop {
            sc.space();
            if !sc.take("&") {
                return Ok(out);
            }
            // `b & b` is one node, not two links side by side.
            let node = self.node(sc, sub)?;
            if !out.contains(&node) {
                out.push(node);
            }
        }
    }

    fn node(&mut self, sc: &mut Scan, sub: Option<usize>) -> Result<usize, String> {
        sc.space();
        let id = sc.id();
        if id.is_empty() {
            return Err(format!("expected a node at `{}`", sc.rest_shown()));
        }
        let at = match self.ids.get(id) {
            Some(at) => *at,
            None if self.nodes.len() >= MOST_NODES => return Err(format!("more than {MOST_NODES} nodes")),
            None => {
                let (id, label) = (id.to_string(), id.to_string());
                self.ids.insert(id.clone(), self.nodes.len());
                self.nodes.push(FNode { id, label, shape: Shape::Rect, sub: None });
                self.nodes.len() - 1
            }
        };
        if let Some((open, close, shape)) = OPENERS.iter().find(|opener| sc.rest().starts_with(opener.0)) {
            let body = &sc.rest()[open.len()..];
            // What is in quotes may hold what closes the shape.
            let quoted = match body.strip_prefix('"') {
                Some(inside) => 1 + inside.find('"').ok_or("unclosed quote")?,
                None => 0,
            };
            let end =
                body[quoted..].find(close).ok_or_else(|| format!("`{open}` without `{close}`"))? + quoted;
            let mut label = decode(&body[..end]);
            // The slanted boxes are drawn as boxes: `[/text/]`.
            if *shape == Shape::Rect && label.len() >= 2 && label.starts_with(['/', '\\']) {
                let last = label.char_indices().next_back().map_or(1, |(at, _)| at);
                label = label[1..last].to_string();
            }
            (self.nodes[at].label, self.nodes[at].shape) = (label, *shape);
            sc.skip(open.len() + end + close.len());
        }
        if sc.take(":::") {
            sc.class(); // a class: its name says nothing here
        }
        self.nodes[at].sub = self.nodes[at].sub.or(sub);
        Ok(at)
    }
}

/// What closes a link that has its label in it: `-- text -->`.
const CLOSERS: [&str; 8] = ["-->", "---", ".->", ".-", "==>", "===", "--o", "--x"];

/// A link where the scanner is; none when what is there is no link.
fn link(sc: &mut Scan) -> Option<Link> {
    let tail = if sc.take("<") { Head::Arrow } else { Head::None };
    let (mut body, mut head) = sc.link_body();
    if body.len() < 2 {
        return None;
    }
    let (mut label, mut length) = (String::new(), body);
    let mut stroke = stroke_of(body);
    if head == Head::None && matches!(body, "--" | "==" | "-.") && sc.rest().starts_with(' ') {
        // The label goes up to the part that closes the link.
        let end = sc.nearest(&CLOSERS)?;
        label = decode(&sc.rest()[..end]);
        sc.skip(end);
        (body, head) = sc.link_body();
        (length, stroke) = (body, if stroke == Stroke::Solid { stroke_of(body) } else { stroke });
    }
    sc.space();
    if sc.take("|") {
        let end = sc.rest().find('|')?;
        label = decode(&sc.rest()[..end]);
        sc.skip(end + 1);
    }
    let span = match stroke {
        Stroke::Dotted => length.matches('.').count(),
        _ => length.len().saturating_sub(if head == Head::None { 2 } else { 1 }),
    };
    Some(Link { label, stroke, head, tail, span: span.clamp(1, LONGEST_LINK) })
}

fn stroke_of(body: &str) -> Stroke {
    if body.contains('.') {
        Stroke::Dotted
    } else if body.contains('=') {
        Stroke::Thick
    } else {
        Stroke::Solid
    }
}
