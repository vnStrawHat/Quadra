//! The select that is searched (behaviour/15, "Select"): a field over the options narrows them
//! as it is typed into, the best match first; the arrows move over what is left, and Enter or
//! a click takes one (`quadra_base::pick`).

use std::rc::Rc;

use quadra_base::pick::{Found, Parts};
use quadra_layout::{Align, Length};
use quadra_reactive::{Signal, memo, signal};
use quadra_tree::State;
use quadra_ui::{Div, column, div, row, scroll, text, text_input};

use crate::icon::icon;
use crate::palette::{best, fuzzy_hits, marked};
use crate::{icons, token};

/// The height of an option, and the most of the list that shows.
const ROW: f32 = 23.0;
const MOST: f32 = 280.0;

/// What the popup of a select that is searched holds, `width` wide.
pub(super) fn searched(
    options: Rc<Vec<String>>,
    chosen: Signal<usize>,
    open: Signal<bool>,
    width: f32,
    ask: String,
) -> Div {
    let query = signal(String::new());
    let known = options.clone();
    let found = Found::new(memo(move || best(&query.get(), known.iter().map(String::as_str))));
    let matches = found.matches;
    let take = move |k: usize| {
        if let Some(&i) = matches.get().get(k) {
            chosen.set(i);
            open.set(false); // the focus goes back to the select
        }
    };
    let on = found.on;
    let face = move |_: usize, i: usize| {
        let hits = fuzzy_hits(&query.get_untracked(), &options[i]).unwrap_or_default();
        let line = row().align_items(Align::Center).gap(8.0).grow(1.0).height(ROW).padding_x(6.0);
        let line = line.radius(4.0).label(options[i].clone());
        let line = line.hover(|s| s.bg(token::PANEL_HOVER)).when(State::SELECTED, |s| s.bg(token::PRESS));
        let line = line.child(marked(&options[i], &hits)).child(div().grow(1.0));
        match i == chosen.get_untracked() {
            true => line.child(icon(icons::CHECK).size(12.0).color(token::TEXT_2)),
            false => line,
        }
    };
    let parts = Parts {
        list: Rc::new(|| column().grow(1.0).padding_y(4.0)),
        line: Some(Rc::new(|| div().padding_x(4.0))),
        row: Rc::new(face),
        empty: Some(Rc::new(|| {
            div().padding_x(10.0).padding_y(4.0).child(text("No match").color(token::TEXT_2))
        })),
    };
    // With nothing typed the chosen option is the one the arrows start from; with a query,
    // the best match.
    let start = move |matches: &[usize]| {
        let empty = query.get_untracked().is_empty();
        matches.iter().position(|&i| empty && i == chosen.get_untracked()).unwrap_or(0)
    };
    let rows = found.rows(parts, start, take);
    let field = text_input(query).placeholder(&ask).on_submit(move |_| take(on.get_untracked()));
    let head = row().align_items(Align::Center).gap(6.0).height(32.0).padding_x(8.0).shrink(0.0);
    let head = head.border_bottom(1.0).border_color(token::LINE);
    let head = head.child(icon(icons::SEARCH).size(12.0).color(token::TEXT_3));
    let head = head.child(div().grow(1.0).child(field));
    let tall = move || Length::Px((matches.get().len().max(1) as f32 * ROW + 8.0).min(MOST));
    let list = scroll(signal(0.0), rows).container(div().height(tall));
    let panel = column().width(width).selection_color(token::SELECTION);
    found.keys(panel, take).child(head).child(list)
}
