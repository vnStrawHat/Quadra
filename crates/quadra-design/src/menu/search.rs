//! The searched menu (specs/behaviour/15, "Menu"): the select's search head over a menu's rows,
//! which are asked again as the field changes; Enter does the first row that can be done.

use std::rc::Rc;

use quadra_layout::Align;
use quadra_reactive::{Signal, signal};
use quadra_view::{AnyView, NodeRef, View, by_text, column, div, dynamic, row, text_input};

use super::{Ends, MenuItem, rows};
use crate::icon::icon;
use crate::popup::popover;
use crate::{icons, token};

/// A menu below `opener` while `open` is true, with a field over its rows that says `ask`
/// while empty; `items` is given what the field says. Each opening starts empty.
pub fn searched_menu<F>(open: Signal<bool>, opener: &NodeRef, ask: &str, items: F) -> impl View + use<F>
where
    F: Fn(&str) -> Vec<MenuItem> + 'static,
{
    let (items, ask) = (Rc::new(items), ask.to_string());
    let ends = move || Ends { done: Rc::new(move || open.set(false)), leave: None };
    popover(open, opener, move || {
        let query = signal(String::new());
        let (listed, first) = (items.clone(), items.clone());
        let field = text_input(query).placeholder(&ask).on_submit(move |typed: &str| {
            let doable = first(typed).into_iter().find(|it| !it.disabled && it.select.is_some());
            if let Some(select) = doable.and_then(|it| it.select) {
                // What the row does first: closing disposes the popup's scope.
                select();
                open.set(false);
            }
        });
        let head =
            row().align_items(Align::Center).gap(6.0).height(by_text(32.0)).padding_x(10.0).shrink(0.0);
        let head = head.border_bottom(1.0).border_color(token::LINE);
        let head = head
            .child(icon(icons::SEARCH).size(12.0).color(token::TEXT_3))
            .child(div().grow(1.0).child(field));
        let body = dynamic(move || AnyView::new(rows(listed(&query.get()), ends())));
        column().min_width(280.0).selection_color(token::SELECTION).child(head).child(body.container(div()))
    })
}

/// A ghost button that is only an icon and opens a searched menu under itself.
pub fn searched_icon_button(
    body: &'static str,
    name: &'static str,
    ask: &str,
    items: impl Fn(&str) -> Vec<MenuItem> + 'static,
) -> super::MenuButton {
    let (items, ask) = (Rc::new(items), ask.to_string());
    let under = move |open: Signal<bool>, opener: &NodeRef| {
        let items = items.clone();
        AnyView::new(searched_menu(open, opener, &ask, move |typed: &str| items(typed)))
    };
    super::MenuButton { face: crate::button::icon_button(body, name), under: Rc::new(under) }
}
