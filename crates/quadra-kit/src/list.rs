//! The list (specs/behaviour/15, "List row"): the base's (`quadra_base::list`) with the
//! kit's rows: 23 px unless told, 6 px beside what they hold, filled under the pointer and
//! when selected.

use std::rc::Rc;

use quadra_base::list as base;
use quadra_layout::Align;
use quadra_reactive::Signal;
use quadra_tree::{NodeId, State};
use quadra_ui::{MountCx, View, column, row, text};

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
        let (row_height, build) = (self.row_height, self.row);
        let line = move |index: Signal<usize>| {
            let line = row().align_items(Align::Center).grow(1.0).height(row_height).padding_x(6.0);
            let line = line.hover(|s| s.bg(token::PANEL_HOVER)).when(State::SELECTED, |s| s.bg(token::PRESS));
            line.child(build(index))
        };
        let frame = column().grow(1.0).focus_visible(|s| s.outline(token::BORDER_FOCUSED));
        let list = base::list(self.count, self.selected, row_height, line).frame(frame);
        let list = match self.activate {
            Some(activate) => list.on_activate(move |at| activate(at)),
            None => list,
        };
        let list = match self.empty {
            Some(words) => list.empty(move || {
                let line = row().padding(12.0).justify(quadra_layout::Justify::Center);
                line.text_color(token::TEXT_2).child(text(words.clone()))
            }),
            None => list,
        };
        list.mount(cx)
    }
}
