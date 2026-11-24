//! The select that is searched (specs/design, "Select"): a field over the options narrows them
//! as it is typed into, the best match first; the arrows move over what is left, and Enter or
//! a click takes one (`quadra_headless::pick`).

use std::rc::Rc;

use quadra_headless::pick::{Found, Parts};
use quadra_layout::{Align, Length};
use quadra_reactive::{Signal, memo, signal};
use quadra_tree::State;
use quadra_view::{Div, by_text, column, div, row, scroll, text, text_input};

use crate::icon::icon;
use crate::palette::{best, fuzzy_hits, marked};
use crate::{icons, token};

/// The height of an option, and the most of the list that shows.
const ROW: f32 = 24.0;
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
        let line = row()
            .align_items(Align::Center)
            .gap(token::SPACE_MD)
            .grow(1.0)
            .height(by_text(ROW))
            .padding_x(token::SPACE_SM);
        let line = line.radius(token::RADIUS_SM).label(options[i].clone());
        let line = line.hover(|s| s.bg(token::PANEL_HOVER)).when(State::SELECTED, |s| s.bg(token::PRESS));
        let line = line.child(marked(&options[i], &hits)).child(div().grow(1.0));
        match i == chosen.get_untracked() {
            true => line.child(icon(icons::CHECK).size(12.0).color(token::TEXT_2)),
            false => line,
        }
    };
    let parts = Parts {
        head: None,
        list: Rc::new(|| column().grow(1.0).padding_y(token::SPACE_XS)),
        line: Some(Rc::new(|| div().padding_x(token::SPACE_XS))),
        row: Rc::new(face),
        empty: Some(Rc::new(|| {
            div()
                .padding_x(token::SPACE_LG)
                .padding_y(token::SPACE_XS)
                .child(text("No match").color(token::TEXT_2))
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
    let head = row()
        .align_items(Align::Center)
        .gap(token::SPACE_SM)
        .height(by_text(32.0))
        .padding_x(token::SPACE_MD)
        .shrink(0.0);
    let head = head.border_bottom(1.0).border_color(token::LINE);
    let head = head.child(icon(icons::SEARCH).size(12.0).color(token::TEXT_3));
    let head = head.child(div().grow(1.0).child(field));
    let row_px = by_text(ROW);
    let tall = move || Length::Px((matches.get().len().max(1) as f32 * row_px() + 8.0).min(MOST));
    let list = scroll(signal(0.0), rows).container(div().height(tall));
    let panel = column().width(width).selection_color(token::SELECTION);
    found.keys(panel, take).child(head).child(list)
}
