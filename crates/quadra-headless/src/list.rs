//! The list (specs/17, "The components"): any number of rows of one height, of which one may
//! be selected. Only the rows that show are mounted (`uniform_list`); the selected row is kept
//! in view. A click selects, a double click or Enter activates; with the focus in the list
//! the arrows move the selection and stop at the ends, Home and End go to them. The look
//! gives the frame, each row's face and what an empty list says.

use std::rc::Rc;

use quadra_reactive::{Signal, memo, signal};
use quadra_tree::{NodeId, Role};
use quadra_view::{AnyView, Div, MountCx, View, column, div, show, uniform_list};

use crate::action::{Confirm, LIST, MoveFirst, MoveLast, MoveNext, MovePrevious};

type Activate = Rc<dyn Fn(usize)>;

pub struct List<F> {
    count: Box<dyn Fn() -> usize>,
    selected: Signal<Option<usize>>,
    row: F,
    row_height: f32,
    frame: Div,
    activate: Option<Activate>,
    empty: Option<Rc<dyn Fn() -> AnyView>>,
}

/// A list of `count` rows, each `row_height` tall. `row` builds a row's face from its index,
/// a signal: a row is used again for another index as the list scrolls. `selected` holds
/// which row is, if any; the row is `State::SELECTED` while it is.
pub fn list<F>(
    count: impl Fn() -> usize + 'static,
    selected: Signal<Option<usize>>,
    row_height: f32,
    row: F,
) -> List<F>
where
    F: Fn(Signal<usize>) -> Div + 'static,
{
    let (frame, activate, empty) = (column(), None, None);
    List { count: Box::new(count), selected, row, row_height, frame, activate, empty }
}

impl<F> List<F> {
    /// The box the rows are in; a column unless told.
    pub fn frame(mut self, frame: Div) -> Self {
        self.frame = frame;
        self
    }

    /// Runs `f` with a row's index when it is chosen for good: a double click, or Enter.
    pub fn on_activate(mut self, f: impl Fn(usize) + 'static) -> Self {
        self.activate = Some(Rc::new(f));
        self
    }

    /// What the list shows, above where its rows go, while it has none.
    pub fn empty<V: View + 'static>(mut self, view: impl Fn() -> V + 'static) -> Self {
        self.empty = Some(Rc::new(move || AnyView::new(view())));
        self
    }
}

impl<F> View for List<F>
where
    F: Fn(Signal<usize>) -> Div + 'static,
{
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (selected, row_height, build) = (self.selected, self.row_height, self.row);
        let count = memo(self.count);
        let activate = self.activate.unwrap_or_else(|| Rc::new(|_| {}));
        let chosen = activate.clone();
        let line = move |index: Signal<usize>| {
            let chosen = chosen.clone();
            let line = build(index).role(Role::ListItem).place_in_set(move || index.get());
            let line = line.selected(move || selected.get() == Some(index.get()));
            line.on_click(move |e| {
                let at = index.get_untracked();
                selected.set(Some(at));
                if e.clicks == 2 {
                    chosen(at);
                }
            })
        };
        let rows = uniform_list(move || count.get(), row_height, signal(0.0), line);
        let rows = rows.container(div().grow(1.0)).reveal(move || selected.get());

        let go = move |to: fn(Option<usize>, usize) -> usize| {
            if let Some(last) = count.get().checked_sub(1) {
                selected.update(|at| *at = Some(to(*at, last)));
            }
        };
        let frame = self.frame.role(Role::List).set_size(move || count.get()).focusable().key_context(LIST);
        let frame = frame.on_action::<MoveNext>(move |_| go(|at, last| at.map_or(0, |i| (i + 1).min(last))));
        let frame =
            frame.on_action::<MovePrevious>(move |_| go(|at, _| at.map_or(0, |i| i.saturating_sub(1))));
        let frame = frame.on_action::<MoveFirst>(move |_| go(|_, _| 0));
        let frame = frame.on_action::<MoveLast>(move |_| go(|_, last| last));
        let frame = frame.on_action::<Confirm>(move |_| {
            if let Some(at) = selected.get_untracked() {
                activate(at);
            }
        });
        let frame = match self.empty {
            Some(empty) => frame.child(show(move || count.get() == 0, move || empty())),
            None => frame,
        };
        frame.child(rows).mount(cx)
    }
}

/// The box of a few places of which `chosen` holds the one chosen, by its id: a sidebar. It
/// takes the focus; the arrows move the choice along the places `ids` gives (those that show,
/// asked at each key) and stop at the ends, Home and End go to them. A tree to a reader.
pub fn places<T: Copy + PartialEq + 'static>(
    frame: Div,
    ids: impl Fn() -> Vec<T> + 'static,
    chosen: Signal<T>,
) -> Div {
    let ids = Rc::new(ids);
    let go = move |to: fn(Option<usize>, usize) -> usize| {
        let ids = quadra_reactive::untrack(|| ids());
        let (at, last) = (ids.iter().position(|id| *id == chosen.get_untracked()), ids.len().checked_sub(1));
        if let Some(last) = last {
            chosen.set(ids[to(at, last)]);
        }
    };
    let (next, previous, first, last) = (go.clone(), go.clone(), go.clone(), go);
    let frame = frame.role(Role::Tree).focusable().key_context(LIST);
    let frame = frame.on_action::<MoveNext>(move |_| next(|at, last| at.map_or(0, |i| (i + 1).min(last))));
    let frame =
        frame.on_action::<MovePrevious>(move |_| previous(|at, _| at.map_or(0, |i| i.saturating_sub(1))));
    let frame = frame.on_action::<MoveFirst>(move |_| first(|_, _| 0));
    frame.on_action::<MoveLast>(move |_| last(|_, last| last))
}

/// The place `id` on its face: `State::SELECTED` while it is the one chosen, chosen by a
/// click. An item of the tree to a reader.
pub fn place<T: Copy + PartialEq + 'static>(face: Div, id: T, chosen: Signal<T>) -> Div {
    face.role(Role::TreeItem).selected(move || chosen.get() == id).on_click(move |_| chosen.set(id))
}
