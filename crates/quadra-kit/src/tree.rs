//! The tree (specs/behaviour/09, "Data"): things inside things, as rows that open and close.
//! The rows that show are mounted; one is selected, by the pointer or the keys.

use std::collections::HashSet;
use std::rc::Rc;

use quadra_layout::Align;
use quadra_platform::Cursor;
use quadra_reactive::{Signal, memo, signal};
use quadra_tree::{NodeId, Paint, Role, Side, State};
use quadra_ui::{Div, MountCx, View, column, div, dynamic, row, text, uniform_list};

use crate::action::{Confirm, MoveFirst, MoveInto, MoveLast, MoveNext, MoveOut, MovePrevious, TREE};
use crate::display::chosen_mark;
use crate::icon::icon;
use crate::{icons, token};

/// How far a level is in from the one around it, and the height of a row, unless told.
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

    /// Something after the label: a value, a count. Of what holds things it shows only
    /// while that is closed: it says in short what is not seen.
    pub fn detail(mut self, text: impl Into<String>, color: impl Into<Paint>) -> Self {
        self.detail = Some((text.into(), color.into()));
        self
    }
}

/// A row: a thing that shows, how deep it is, and the row of what it is in.
#[derive(Clone, Debug, PartialEq)]
struct Shown {
    id: String,
    label: String,
    icon: Option<&'static str>,
    color: Option<Paint>,
    detail: Option<(String, Paint)>,
    depth: usize,
    parent: Option<usize>,
    /// Whether it holds anything, and whether it is open.
    holds: bool,
    open: bool,
}

fn walk(
    nodes: &[TreeNode],
    open: &HashSet<String>,
    depth: usize,
    parent: Option<usize>,
    out: &mut Vec<Shown>,
) {
    for node in nodes {
        let holds = !node.children.is_empty();
        let (id, label, is_open) = (node.id.clone(), node.label.clone(), holds && open.contains(&node.id));
        let (icon, color, detail) = (node.icon, node.color, node.detail.clone().filter(|_| !is_open));
        out.push(Shown { id, label, icon, color, detail, depth, parent, holds, open: is_open });
        if is_open {
            walk(&node.children, open, depth + 1, Some(out.len() - 1), out);
        }
    }
}

pub struct Tree {
    roots: Box<dyn Fn() -> Vec<TreeNode>>,
    chosen: Signal<Option<String>>,
    open: Option<Signal<HashSet<String>>>,
    /// The height of a row, and how far a level is in.
    sizes: (f32, f32),
}

/// A tree of what `roots` gives, asked again when what it reads changes. `chosen` holds the
/// id of the selected thing, if any. Give the tree a height, or let it grow.
pub fn tree(roots: impl Fn() -> Vec<TreeNode> + 'static, chosen: Signal<Option<String>>) -> Tree {
    Tree { roots: Box::new(roots), chosen, open: None, sizes: (ROW, INDENT) }
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
}

/// What a row shows: guides for the levels it is in, a chevron if it holds something, its
/// icon, its label and its detail; and while `on`, the accent's mark at its left and its icon
/// in the accent, unless it has a colour of its own.
fn line(at: &Shown, (height, indent): (f32, f32), on: impl Fn() -> bool + Clone + 'static) -> Div {
    let left = 8.0 + at.depth as f32 * indent;
    let mut line = row().align_items(Align::Center).gap(2.0).grow(1.0).height(height);
    line = line.padding_left(left).padding_right(8.0).cursor(Cursor::Pointer);
    line = line.role(Role::TreeItem).label(at.label.clone());
    for level in 0..at.depth {
        let guide = div().absolute().left(8.0 + level as f32 * indent + 7.0).top(0.0).bottom(0.0);
        line = line.child(guide.width(1.0).bg(token::TEXT.alpha(0.1)));
    }
    line = match at.holds {
        true => {
            let body = if at.open { icons::CHEVRON_DOWN } else { icons::CHEVRON_RIGHT };
            line.expanded(at.open).child(icon(body).size(16.0).color(token::TEXT_2))
        }
        false => line.child(div().width(16.0)),
    };
    if let Some(body) = at.icon {
        let (own, lit) = (at.color, on.clone());
        let tone = move || own.unwrap_or(if lit() { token::ACCENT_TEXT } else { token::TEXT_2 }.into());
        line = line.child(div().text_color(tone).child(icon(body).size(16.0)));
    }
    line = line.child(chosen_mark(Side::Left, on));
    let label = text(at.label.clone());
    let label = at.color.map_or_else(|| text(at.label.clone()), |color| label.color(color));
    let line = line.child(div().padding_left(4.0).child(label));
    match &at.detail {
        Some((detail, color)) => {
            line.child(div().padding_left(6.0).child(text(detail.clone()).color(*color)))
        }
        None => line,
    }
}

impl View for Tree {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (roots, chosen, focused, sizes) = (self.roots, self.chosen, signal(false), self.sizes);
        let open = self.open.unwrap_or_else(|| signal(HashSet::new()));
        let shown = memo(move || {
            let mut all = Vec::new();
            open.with(|open| walk(&roots(), open, 0, None, &mut all));
            Rc::new(all)
        });
        let place = move || {
            let (all, chosen) = (shown.get(), chosen.get());
            chosen.and_then(|id| all.iter().position(|at| at.id == id))
        };
        let toggle = move |id: &str| {
            open.update(|open| {
                if !open.remove(id) {
                    open.insert(id.to_string());
                }
            })
        };
        let rows = uniform_list(
            move || shown.get().len(),
            sizes.0,
            signal(0.0),
            move |at| {
                // A row is built again when the thing at its place is another one, or opens.
                let here = memo(move || shown.get().get(at.get()).cloned());
                dynamic(move || {
                    let Some(at) = here.get() else { return div() };
                    let id = at.id.clone();
                    let on = move || chosen.with(|chosen| chosen.as_deref() == Some(id.as_str()));
                    let (is, ring, id) = (on.clone(), on.clone(), at.id.clone());
                    let line = line(&at, sizes, on.clone()).selected(on);
                    let line = line
                        .hover(|s| s.bg(token::RAISED_HOVER))
                        .when(State::SELECTED, |s| s.bg(token::PRESS));
                    // The selected row of a tree with the focus is outlined.
                    let line = line.outline(move || -> Paint {
                        if focused.get() && ring() { token::BORDER_FOCUSED.into() } else { Paint::default() }
                    });
                    // A click selects; on what holds something, a second click or a double one
                    // opens or closes it.
                    line.on_click(move |e| {
                        if at.holds && (e.clicks == 2 || is()) {
                            toggle(&id);
                        }
                        chosen.set(Some(id.clone()));
                    })
                })
                .container(div().grow(1.0))
            },
        );
        let rows = rows.container(div().grow(1.0)).reveal(place);

        let go = move |to: &dyn Fn(usize, &[Shown]) -> Option<usize>| {
            let all = shown.get();
            let next = match place() {
                Some(at) => to(at, &all),
                None => (!all.is_empty()).then_some(0),
            };
            if let Some(at) = next.and_then(|at| all.get(at)) {
                chosen.set(Some(at.id.clone()));
            }
        };
        let frame = column().grow(1.0).padding_y(4.0).role(Role::Tree).focusable().key_context(TREE);
        let frame = frame.on_focus(move |has| focused.set(has));
        let frame = frame.on_action::<MoveNext>(move |_| go(&|at, all| Some((at + 1).min(all.len() - 1))));
        let frame = frame.on_action::<MovePrevious>(move |_| go(&|at, _| Some(at.saturating_sub(1))));
        let frame = frame.on_action::<MoveFirst>(move |_| go(&|_, _| Some(0)));
        let frame = frame.on_action::<MoveLast>(move |_| go(&|_, all| all.len().checked_sub(1)));
        // Right opens what is closed, then goes into it; left closes what is open, then goes
        // out to what holds it; Enter and Space open and close.
        let frame = frame.on_action::<MoveInto>(move |_| {
            go(&|at, all| match (all[at].holds, all[at].open) {
                (true, false) => {
                    toggle(&all[at].id);
                    Some(at)
                }
                (true, true) => Some(at + 1),
                _ => None,
            })
        });
        let frame = frame.on_action::<MoveOut>(move |_| {
            go(&|at, all| {
                if !all[at].open {
                    return all[at].parent;
                }
                toggle(&all[at].id);
                Some(at)
            })
        });
        let frame = frame.on_action::<Confirm>(move |_| {
            go(&|at, all| {
                if all[at].holds {
                    toggle(&all[at].id);
                }
                Some(at)
            })
        });
        frame.child(rows).mount(cx)
    }
}
