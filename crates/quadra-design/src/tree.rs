//! The tree (specs/behaviour/09, "Data"): things inside things, as rows that open and close.
//! The rows that show are mounted; one is selected, by the pointer or the keys.

use std::collections::HashSet;

use quadra_headless::tree::{self, Branch, Shown};
use quadra_layout::Align;
use quadra_platform::Cursor;
use quadra_reactive::{Signal, signal};
use quadra_tree::{NodeId, Paint, Side, State};
use quadra_view::{Div, MountCx, View, by_text, column, div, row, text};

use crate::display::chosen_mark;
use crate::icon::icon;
use crate::{icons, token};

/// How far a level is in from the one around it, and the height of a row, unless told.
/// What is told the id of a thing that opens.
type Told = Box<dyn Fn(&str)>;

const INDENT: f32 = 20.0;
const ROW: f32 = 26.0;

/// A thing of a tree: what it is known by, what it is called, and what is in it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreeNode {
    pub id: String,
    pub label: String,
    pub icon: Option<&'static str>,
    pub children: Vec<TreeNode>,
    /// The colour of the label and the icon, when it is not that of the text around: a hue
    /// for a kind (`token::HUE_YELLOW` for what was changed).
    pub color: Option<Paint>,
    /// What follows the label, and its colour.
    pub detail: Option<(String, Paint)>,
    /// What follows the detail while it is closed: what is not seen, in short.
    pub folded: Option<(String, Paint)>,
    /// It holds things that are given when it opens (`Tree::on_open`).
    pub lazy: bool,
}

impl TreeNode {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> TreeNode {
        TreeNode { id: id.into(), label: label.into(), ..TreeNode::default() }
    }

    pub fn icon(mut self, body: &'static str) -> Self {
        self.icon = Some(body);
        self
    }

    pub fn children(mut self, inside: impl IntoIterator<Item = TreeNode>) -> Self {
        self.children = inside.into_iter().collect();
        self
    }

    /// A colour for the label, or a role of the theme.
    pub fn color(mut self, color: impl Into<Paint>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Something after the label: a value, a mark.
    pub fn detail(mut self, text: impl Into<String>, color: impl Into<Paint>) -> Self {
        self.detail = Some((text.into(), color.into()));
        self
    }

    /// What it says after its detail while it is closed: in short what is not seen (`{3}`).
    pub fn folded(mut self, text: impl Into<String>, color: impl Into<Paint>) -> Self {
        self.folded = Some((text.into(), color.into()));
        self
    }

    /// It holds things not given yet: it shows as closed, and the tree's `on_open` is told
    /// when it opens, to give them.
    pub fn lazy(mut self) -> Self {
        self.lazy = true;
        self
    }
}

/// What a row of a thing shows (the headless crate walks the things: `quadra_headless::tree`).
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    label: String,
    icon: Option<&'static str>,
    color: Option<Paint>,
    detail: Option<(String, Paint)>,
    folded: Option<(String, Paint)>,
}

impl Branch for TreeNode {
    type Row = Look;

    fn id(&self) -> &str {
        &self.id
    }

    fn children(&self) -> &[TreeNode] {
        &self.children
    }

    fn lazy(&self) -> bool {
        self.lazy && self.children.is_empty()
    }

    fn row(&self, open: bool) -> Look {
        let (detail, folded) = (self.detail.clone(), self.folded.clone().filter(|_| !open));
        Look { label: self.label.clone(), icon: self.icon, color: self.color, detail, folded }
    }
}

pub struct Tree {
    roots: Box<dyn Fn() -> Vec<TreeNode>>,
    chosen: Signal<Option<String>>,
    open: Option<Signal<HashSet<String>>>,
    /// The height of a row, and how far a level is in.
    sizes: (f32, f32),
    on_open: Option<Told>,
    many: Option<Signal<Vec<String>>>,
}

/// A tree of what `roots` gives, asked again when what it reads changes. `chosen` holds the
/// id of the selected thing, if any. Give the tree a height, or let it grow.
pub fn tree(roots: impl Fn() -> Vec<TreeNode> + 'static, chosen: Signal<Option<String>>) -> Tree {
    Tree { roots: Box::new(roots), chosen, open: None, sizes: (ROW, INDENT), on_open: None, many: None }
}

impl Tree {
    /// The signal that holds the ids of the things that are open: the caller's, to keep.
    pub fn open(mut self, ids: Signal<HashSet<String>>) -> Self {
        self.open = Some(ids);
        self
    }

    pub fn row_height(mut self, px: f32) -> Self {
        self.sizes.0 = px;
        self
    }

    /// How far a level is in from the one around it.
    pub fn indent(mut self, px: f32) -> Self {
        self.sizes.1 = px;
        self
    }

    /// Told the id of a thing when it opens: to give what a lazy one holds.
    pub fn on_open(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.on_open = Some(Box::new(f));
        self
    }

    /// Several things are selected, their ids in `selected`: Ctrl and a click adds or takes
    /// one away, Shift and a click selects from the one the keys are on.
    pub fn many(mut self, selected: Signal<Vec<String>>) -> Self {
        self.many = Some(selected);
        self
    }
}

/// What a row shows: guides for the levels it is in, a chevron if it holds something, its
/// icon, its label and its detail; and while `on`, the accent's mark at its left and its icon
/// in the accent, unless it has a colour of its own.
fn line(at: &Shown<Look>, (height, indent): (f32, f32), on: impl Fn() -> bool + Clone + 'static) -> Div {
    let left = 8.0 + at.depth as f32 * indent;
    let mut line = row().align_items(Align::Center).gap(2.0).grow(1.0).height(by_text(height));
    line = line.padding_left(left).padding_right(8.0).cursor(Cursor::Pointer);
    line = line.label(at.row.label.clone());
    for level in 0..at.depth {
        let guide = div().absolute().left(8.0 + level as f32 * indent + 7.0).top(0.0).bottom(0.0);
        line = line.child(guide.width(1.0).bg(token::TEXT.alpha(0.1)));
    }
    line = match at.holds {
        true => {
            let body = if at.open { icons::CHEVRON_DOWN } else { icons::CHEVRON_RIGHT };
            line.child(icon(body).size(16.0).color(token::TEXT_2))
        }
        false => line.child(div().width(16.0)),
    };
    if let Some(body) = at.row.icon {
        let (own, lit) = (at.row.color, on.clone());
        let tone = move || own.unwrap_or(if lit() { token::ACCENT_TEXT } else { token::TEXT_2 }.into());
        line = line.child(div().text_color(tone).child(icon(body).size(16.0)));
    }
    line = line.child(chosen_mark(Side::Left, on));
    // One line: a row is as high as one, and its label ends with "…" where it does not fit.
    let label = text(at.row.label.clone()).ellipsis();
    let label = match at.row.color {
        Some(color) => label.color(color),
        None => label,
    };
    let line = line.child(div().padding_left(4.0).child(label));
    let after =
        |(words, color): &(String, Paint)| div().padding_left(6.0).child(text(words.clone()).color(*color));
    let line = match &at.row.detail {
        Some(detail) => line.child(after(detail)),
        None => line,
    };
    match &at.row.folded {
        Some(folded) => line.child(after(folded)),
        None => line,
    }
}

impl View for Tree {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let sizes = self.sizes;
        let open = self.open.unwrap_or_else(|| signal(HashSet::new()));
        let face = move |at: &Shown<Look>, state: tree::Line| {
            let (on, focused, ring) = (state.on, state.focused, state.here);
            let lit = on.clone();
            let line = line(at, sizes, move || lit());
            let line =
                line.hover(|s| s.bg(token::RAISED_HOVER)).when(State::SELECTED, |s| s.bg(token::PRESS));
            // The selected row of a tree with the focus is outlined.
            line.outline(move || -> Paint {
                if focused.get() && ring() { token::BORDER_FOCUSED.into() } else { Paint::default() }
            })
        };
        let frame = column().grow(1.0).padding_y(4.0);
        let base = tree::tree(self.roots, self.chosen, open, sizes.0, face).frame(frame);
        let base = match self.on_open {
            Some(told) => base.on_open(told),
            None => base,
        };
        let base = match self.many {
            Some(many) => base.many(many),
            None => base,
        };
        base.mount(cx)
    }
}
