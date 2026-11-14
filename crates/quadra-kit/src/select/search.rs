//! The select that is searched (behaviour/15, "Select"): a field over the options narrows them
//! as it is typed into, the best match first; the arrows move over what is left, and Enter or
//! a click takes one.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_layout::{Align, Length};
use quadra_reactive::{Signal, memo, signal};
use quadra_tree::{Role, State};
use quadra_ui::{Div, NodeRef, column, div, dynamic, row, scroll, text, text_input};

use crate::action::{Confirm, LIST, MoveNext, MovePrevious};
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
    let (query, on) = (signal(String::new()), signal(0usize));
    let known = options.clone();
    let found = memo(move || best(&query.get(), known.iter().map(String::as_str)));
    let take = move |k: usize| {
        if let Some(&i) = found.get().get(k) {
            chosen.set(i);
            open.set(false); // the focus goes back to the select
        }
    };
    let refs: Rc<RefCell<Vec<NodeRef>>> = Rc::default();
    let lines = refs.clone();
    let rows = dynamic(move || {
        let (matches, query) = (found.get(), query.get_untracked());
        // With nothing typed the chosen option is the one the arrows start from; with a query,
        // the best match.
        let start = matches.iter().position(|&i| query.is_empty() && i == chosen.get_untracked());
        on.set(start.unwrap_or(0));
        *lines.borrow_mut() = matches.iter().map(|_| NodeRef::new()).collect();
        let mut list = column().grow(1.0).padding_y(4.0).role(Role::List);
        for (k, &i) in matches.iter().enumerate() {
            let hits = fuzzy_hits(&query, &options[i]).unwrap_or_default();
            let line = row().align_items(Align::Center).gap(8.0).grow(1.0).height(ROW).padding_x(6.0);
            let line = line.radius(4.0).role(Role::ListItem).label(options[i].clone());
            let line = line.selected(move || on.get() == k).node_ref(&lines.borrow()[k]);
            let line = line.hover(|s| s.bg(token::PANEL_HOVER)).when(State::SELECTED, |s| s.bg(token::PRESS));
            let line =
                line.on_click(move |_| take(k)).child(marked(&options[i], &hits)).child(div().grow(1.0));
            let line = match i == chosen.get_untracked() {
                true => line.child(icon(icons::CHECK).size(12.0).color(token::TEXT_2)),
                false => line,
            };
            list = list.child(div().padding_x(4.0).child(line));
        }
        if matches.is_empty() {
            list =
                list.child(div().padding_x(10.0).padding_y(4.0).child(text("No match").color(token::TEXT_2)));
        }
        list
    });
    // The arrows go round the matches and keep the one they are on in view.
    let step = move |forward: bool| {
        let count = found.get().len();
        if count > 0 {
            on.update(|at| *at = if forward { (*at + 1) % count } else { (*at + count - 1) % count });
            refs.borrow()[on.get_untracked()].scroll_into_view();
        }
    };
    let (down, up) = (step.clone(), step);
    let field = text_input(query).placeholder(&ask).on_submit(move |_| take(on.get_untracked()));
    let head = row().align_items(Align::Center).gap(6.0).height(32.0).padding_x(8.0).shrink(0.0);
    let head = head.border_bottom(1.0).border_color(token::LINE);
    let head = head.child(icon(icons::SEARCH).size(12.0).color(token::TEXT_3));
    let head = head.child(div().grow(1.0).child(field));
    let tall = move || Length::Px((found.get().len().max(1) as f32 * ROW + 8.0).min(MOST));
    let list = scroll(signal(0.0), rows).container(div().height(tall));
    let panel = column().width(width).key_context(LIST).selection_color(token::SELECTION);
    let panel =
        panel.on_action::<MoveNext>(move |_| down(true)).on_action::<MovePrevious>(move |_| up(false));
    panel.on_action::<Confirm>(move |_| take(on.get_untracked())).child(head).child(list)
}
