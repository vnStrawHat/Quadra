//! The combobox (specs/behaviour/09, "Forms"): the headless crate's (`quadra_headless::combo`) with the
//! kit's input, 280 px, and its options on a popup's plane under it: rows of 23 px, at most
//! 240 px of them.

use std::rc::Rc;

use quadra_headless::pick::Parts;
use quadra_layout::{Align, Length};
use quadra_reactive::{Signal, signal};
use quadra_tree::{Along, NodeId, Side, State};
use quadra_view::{IntoProp, MountCx, Place, Prop, View, by_text, column, div, row, scroll};

use crate::input::input;
use crate::palette::{fuzzy_hits, marked};
use crate::popup::surface;
use crate::{icons, token};

/// The height of an option, and the most of the list that shows.
const ROW: f32 = 24.0;
const MOST: f32 = 240.0;

type Pick = Rc<dyn Fn(usize)>;

pub struct Combobox {
    options: Rc<Vec<String>>,
    value: Signal<String>,
    placeholder: String,
    label: Option<Prop<String>>,
    pick: Option<Pick>,
}

/// An input of `value`, 280 px wide, that offers the `options` its text matches.
pub fn combobox<S: Into<String>>(options: impl IntoIterator<Item = S>, value: Signal<String>) -> Combobox {
    let options = Rc::new(options.into_iter().map(Into::into).collect());
    Combobox { options, value, placeholder: String::new(), label: None, pick: None }
}

impl Combobox {
    pub fn placeholder(mut self, words: impl Into<String>) -> Self {
        self.placeholder = words.into();
        self
    }

    /// What a reader calls its input, a constant or words that change.
    pub fn label<M>(mut self, name: impl IntoProp<String, M>) -> Self {
        self.label = Some(name.into_prop());
        self
    }

    /// Runs `f` with the index of an option when it is taken.
    pub fn on_pick(mut self, f: impl Fn(usize) + 'static) -> Self {
        self.pick = Some(Rc::new(f));
        self
    }
}

impl View for Combobox {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (options, value, placeholder) = (self.options.clone(), self.value, self.placeholder);
        let face = move |_: usize, i: usize| {
            let hits = fuzzy_hits(&value.get_untracked(), &options[i]).unwrap_or_default();
            let line =
                row().align_items(Align::Center).grow(1.0).height(by_text(ROW)).padding_x(6.0).radius(4.0);
            let line = line.label(options[i].clone());
            let line = line.hover(|s| s.bg(token::PANEL_HOVER)).when(State::SELECTED, |s| s.bg(token::PRESS));
            line.child(marked(&options[i], &hits))
        };
        let parts = Parts {
            head: None,
            list: Rc::new(|| column().grow(1.0).padding_y(4.0)),
            line: Some(Rc::new(|| div().padding_x(4.0))),
            row: Rc::new(face),
            empty: None,
        };
        let label = self.label;
        let field = move |submit: Rc<dyn Fn()>| {
            let field = input(value).placeholder(&placeholder).icon(icons::SEARCH).own_width(280.0);
            let field = match label {
                Some(name) => field.label(name),
                None => field,
            };
            field.on_submit(move |_| submit())
        };
        let panel = |found: &quadra_headless::pick::Found, rows| {
            let matches = found.matches;
            let row_px = by_text(ROW);
            let tall = move || Length::Px((matches.get().len() as f32 * row_px() + 8.0).min(MOST));
            surface(1).width(280.0).child(scroll(signal(0.0), rows).container(div().height(tall)))
        };
        let place =
            |anchor: &quadra_view::NodeRef| Place::Beside(anchor.clone(), Side::Below, Along::Start, 4.0);
        let combo = quadra_headless::combobox(self.options, value, parts, field, panel, place);
        match self.pick {
            Some(pick) => combo.on_pick(move |i| pick(i)),
            None => combo,
        }
        .mount(cx)
    }
}
