//! The tree (specs/17, "The components"): things inside things, as rows that open and close.
//! The rows that show are mounted (`uniform_list`), and a row is built again only when the
//! thing at its place is another one, or opens. One is selected: a click selects, and on what
//! holds something a second click or a double one opens or closes it. With the focus in the
//! tree the arrows move the selection, Home and End go to the ends, the right arrow opens what
//! is closed then goes into it, the left one closes what is open then goes out to what holds
//! it, and Enter and Space open and close. The look gives each row's face and the frame.

use std::collections::HashSet;
use std::rc::Rc;

use quadra_reactive::{Signal, memo, signal};
use quadra_tree::{NodeId, Role};
use quadra_ui::{Div, MountCx, View, div, dynamic, uniform_list};

use crate::action::{Confirm, MoveFirst, MoveInto, MoveLast, MoveNext, MoveOut, MovePrevious, TREE};

/// A thing of a tree, as the base walks it.
pub trait Branch: Sized + 'static {
    /// What a row of it shows, without what is inside it.
    type Row: Clone + PartialEq + 'static;
    fn id(&self) -> &str;
    fn children(&self) -> &[Self];
    /// What its row shows, open or not.
    fn row(&self, open: bool) -> Self::Row;
}

/// A row that shows: the thing, how deep it is, and the row of what it is in.
#[derive(Clone, Debug, PartialEq)]
pub struct Shown<R> {
    pub id: String,
    pub row: R,
    pub depth: usize,
    pub parent: Option<usize>,
    /// Whether it holds anything, and whether it is open.
    pub holds: bool,
    pub open: bool,
}

fn walk<B: Branch>(
    nodes: &[B],
    open: &HashSet<String>,
    depth: usize,
    parent: Option<usize>,
    out: &mut Vec<Shown<B::Row>>,
) {
    for node in nodes {
        let holds = !node.children().is_empty();
        let is_open = holds && open.contains(node.id());
        let (id, row) = (node.id().to_string(), node.row(is_open));
        out.push(Shown { id, row, depth, parent, holds, open: is_open });
        if is_open {
            walk(node.children(), open, depth + 1, Some(out.len() - 1), out);
        }
    }
}

/// What the face of a row is built with: whether it is the selected one, and whether the
/// tree has the focus.
#[derive(Clone)]
pub struct Line {
    pub on: Rc<dyn Fn() -> bool>,
    pub focused: Signal<bool>,
}

type Face<R> = Rc<dyn Fn(&Shown<R>, Line) -> Div>;
/// Where a key goes from the selected row, among the rows that show.
type Step<'a, R> = dyn Fn(usize, &[Shown<R>]) -> Option<usize> + 'a;

pub struct Tree<B: Branch> {
    roots: Box<dyn Fn() -> Vec<B>>,
    chosen: Signal<Option<String>>,
    open: Signal<HashSet<String>>,
    row_height: f32,
    frame: Div,
    face: Face<B::Row>,
}

/// A tree of what `roots` gives, asked again when what it reads changes, in rows of
/// `row_height`. `chosen` holds the id of the selected thing, `open` those of the things that
/// are open; `face` builds a row's face, which the base makes a tree item named by the look.
pub fn tree<B: Branch>(
    roots: impl Fn() -> Vec<B> + 'static,
    chosen: Signal<Option<String>>,
    open: Signal<HashSet<String>>,
    row_height: f32,
    face: impl Fn(&Shown<B::Row>, Line) -> Div + 'static,
) -> Tree<B> {
    let (roots, face) = (Box::new(roots), Rc::new(face));
    Tree { roots, chosen, open, row_height, frame: quadra_ui::column(), face }
}

impl<B: Branch> Tree<B> {
    /// The box the rows are in; a column unless told.
    pub fn frame(mut self, frame: Div) -> Self {
        self.frame = frame;
        self
    }
}

impl<B: Branch> View for Tree<B> {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (roots, chosen, open, face) = (self.roots, self.chosen, self.open, self.face);
        let focused = signal(false);
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
            self.row_height,
            signal(0.0),
            move |at| {
                let (row_at, face) = (at, face.clone());
                let here = memo(move || shown.get().get(at.get()).cloned());
                dynamic(move || {
                    let Some(at) = here.get() else { return div() };
                    let id = at.id.clone();
                    let on = move || chosen.with(|chosen| chosen.as_deref() == Some(id.as_str()));
                    let (is, id) = (on.clone(), at.id.clone());
                    let line = face(&at, Line { on: Rc::new(on.clone()), focused }).role(Role::TreeItem);
                    // ponytail: a row's place among all the rows that show; its level and its place
                    // among its siblings when a reader needs them.
                    let line = line.selected(on).place_in_set(move || row_at.get());
                    let line = if at.holds { line.expanded(at.open) } else { line };
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

        let go = move |to: &Step<'_, B::Row>| {
            let all = shown.get();
            let next = match place() {
                Some(at) => to(at, &all),
                None => (!all.is_empty()).then_some(0),
            };
            if let Some(at) = next.and_then(|at| all.get(at)) {
                chosen.set(Some(at.id.clone()));
            }
        };
        let frame = self.frame.role(Role::Tree).focusable().key_context(TREE);
        let frame = frame.set_size(move || shown.get().len()).on_focus(move |has| focused.set(has));
        let frame = frame.on_action::<MoveNext>(move |_| go(&|at, all| Some((at + 1).min(all.len() - 1))));
        let frame = frame.on_action::<MovePrevious>(move |_| go(&|at, _| Some(at.saturating_sub(1))));
        let frame = frame.on_action::<MoveFirst>(move |_| go(&|_, _| Some(0)));
        let frame = frame.on_action::<MoveLast>(move |_| go(&|_, all| all.len().checked_sub(1)));
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
