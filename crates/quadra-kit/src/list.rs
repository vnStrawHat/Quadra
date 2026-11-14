//! The list (specs/behaviour/15, "List row"): any number of rows of one height, of which one
//! may be selected. Only the rows that show are mounted (`uniform_list`); the selected row is
//! kept in view.

use std::rc::Rc;

use quadra_layout::Align;
use quadra_reactive::{Signal, memo, signal};
use quadra_tree::{NodeId, Role, State};
use quadra_ui::{MountCx, View, column, div, row, show, text, uniform_list};

use crate::action::{Confirm, LIST, MoveFirst, MoveLast, MoveNext, MovePrevious};
use crate::token;

type Activate = Rc<dyn Fn(usize)>;

pub struct List<F> {
    count: Box<dyn Fn() -> usize>,
    selected: Signal<Option<usize>>,
    row: F,
    row_height: f32,
    activate: Option<Activate>,
    empty: Option<String>,
}

/// A list of `count` rows. `row` builds what a row shows from its index, a signal: a row is
/// used again for another index as the list scrolls. `selected` holds which row is, if any;
/// a click or the arrows write it. Give the list a height, or let it grow.
pub fn list<V, F>(count: impl Fn() -> usize + 'static, selected: Signal<Option<usize>>, row: F) -> List<F>
where
    V: View + 'static,
    F: Fn(Signal<usize>) -> V + 'static,
{
    List { count: Box::new(count), selected, row, row_height: 23.0, activate: None, empty: None }
}

impl<F> List<F> {
    /// The height of every row: 23 px unless told; 32 for the rows of a picker.
    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = height;
        self
    }

    /// Runs `f` with a row's index when it is chosen for good: a double click, or Enter.
    pub fn on_activate(mut self, f: impl Fn(usize) + 'static) -> Self {
        self.activate = Some(Rc::new(f));
        self
    }

    /// What the list says while it has no rows.
    pub fn empty(mut self, words: impl Into<String>) -> Self {
        self.empty = Some(words.into());
        self
    }
}

impl<V, F> View for List<F>
where
    V: View + 'static,
    F: Fn(Signal<usize>) -> V + 'static,
{
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (selected, row_height, build) = (self.selected, self.row_height, self.row);
        let count = memo(self.count);
        let activate = self.activate.unwrap_or_else(|| Rc::new(|_| {}));
        let chosen = activate.clone();
        let line = move |index: Signal<usize>| {
            let chosen = chosen.clone();
            let line = row().align_items(Align::Center).grow(1.0).height(row_height).padding_x(6.0);
            let line = line.role(Role::ListItem).place_in_set(move || index.get());
            let line = line.selected(move || selected.get() == Some(index.get()));
            let line = line.hover(|s| s.bg(token::PANEL_HOVER)).when(State::SELECTED, |s| s.bg(token::PRESS));
            let line = line.on_click(move |e| {
                let at = index.get_untracked();
                selected.set(Some(at));
                if e.clicks == 2 {
                    chosen(at);
                }
            });
            line.child(build(index))
        };
        let rows = uniform_list(move || count.get(), row_height, signal(0.0), line);
        let rows = rows.container(div().grow(1.0)).reveal(move || selected.get());

        // The arrows move the selection and stop at the ends; Home and End go to them.
        let go = move |to: fn(Option<usize>, usize) -> usize| {
            let last = count.get().checked_sub(1);
            if let Some(last) = last {
                selected.update(|at| *at = Some(to(*at, last)));
            }
        };
        let frame = column().grow(1.0).role(Role::List).set_size(move || count.get()).focusable();
        let frame = frame.key_context(LIST);
        let frame = frame.focus_visible(|s| s.outline(token::BORDER_FOCUSED));
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
            Some(words) => {
                let words = move || {
                    let line = row().padding(12.0).justify(quadra_layout::Justify::Center);
                    line.text_color(token::TEXT_2).child(text(words.clone()))
                };
                frame.child(show(move || count.get() == 0, words))
            }
            None => frame,
        };
        frame.child(rows).mount(cx)
    }
}
