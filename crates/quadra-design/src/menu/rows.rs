//! The rows of a menu (specs/behaviour/15, "Menu"): what each looks like, and where the menu
//! a row opens stands.

use std::rc::Rc;

use quadra_headless::menu::{self, Ends, Line};
use quadra_layout::{Align, Justify};
use quadra_reactive::signal;
use quadra_tree::{Along, Side, State};
use quadra_view::{Div, NodeRef, Place, by_text, column, div, row, scroll, text};

use super::MenuItem;
use crate::icon::icon;
use crate::popup::{EDGE, listing, popup};
use crate::{icons, size, token};

/// The height of a row, and the most of a menu that shows before it scrolls.
const ROW: f32 = 28.0;
const MOST: f32 = 450.0;

/// A row that is chosen: 28 px high, 4 px in from the menu's edge, 8 px beside what it holds,
/// 6 px between its parts, the shortcut at the right and 16 px from the label at least.
fn face(item: &MenuItem, marks: bool) -> Div {
    let face = row().height(by_text(ROW)).grow(1.0).padding_x(8.0).gap(6.0).radius(token::RADIUS_SM);
    let face = face.align_items(Align::Center);
    let face = face.hover(|s| s.bg(token::PANEL_HOVER)).when(State::SELECTED, |s| s.bg(token::PRESS));
    let face = if item.danger { face.text_color(token::ERROR) } else { face };
    let face = face.when(State::DISABLED, |s| s.text_color(token::TEXT_DISABLED));
    let face = match (marks, item.checked, item.icon) {
        (false, ..) => face,
        (_, Some(true), _) if item.quiet => face.child(icon(icons::CHECK).color(token::TEXT_2)),
        (_, Some(true), _) => face.child(icon(icons::CHECK).color(token::ACCENT_TEXT)),
        (_, _, Some(body)) if item.disabled => face.child(icon(body)),
        (_, _, Some(body)) => face.child(icon(body).color(token::TEXT_2)),
        _ => match (item.dot, item.swatch) {
            (Some(color), _) => face.child(row().width(14.0).justify(Justify::Center).child(dot(color))),
            (_, Some(color)) => face.child(
                row()
                    .width(14.0)
                    .justify(Justify::Center)
                    .child(div().size(10.0, 10.0).radius(2.0).bg(color)),
            ),
            _ => face.child(div().width(14.0)),
        },
    };
    let face = face.child(text(item.label.clone()));
    let face = match &item.detail {
        Some(words) => face.child(div().text_size(11.0).text_color(token::TEXT_2).child(text(words.clone()))),
        None => face,
    };
    let face = face.child(div().grow(1.0).min_width(16.0));
    // The shortcut in 12 px; disabled, in the row's colour.
    let face = match &item.keys {
        Some(keys) => {
            let keys = div().text_size(size::TEXT_SM.size).child(text(keys.clone()));
            let keys = if item.disabled { keys } else { keys.text_color(token::TEXT_2) };
            face.child(if item.mono { keys.font_family(crate::fonts::MONO) } else { keys })
        }
        None => face,
    };
    match item.items.is_empty() {
        true => face,
        false => face.child(icon(icons::CHEVRON_RIGHT).size(12.0).color(token::TEXT_2)),
    }
}

/// A dot of 7 px.
fn dot(color: quadra_tree::Token) -> Div {
    div().size(7.0, 7.0).radius(3.5).bg(color)
}

/// The rows of a menu as a view: the headless crate's (`quadra_headless::menu`) with the kit's faces.
pub(crate) fn rows(items: Vec<MenuItem>, ends: Ends) -> Div {
    let marks = items.iter().any(|item| {
        item.icon.is_some() || item.dot.is_some() || item.swatch.is_some() || item.checked.is_some()
    });
    let mut lines = Vec::new();
    let mut height = 8.0;
    for item in items {
        if item.separator {
            let rule = div().height(9.0).padding_y(4.0).child(div().grow(1.0).bg(token::LINE_STRONG));
            lines.push(Line::Part(Box::new(rule)));
            height += 9.0;
            continue;
        }
        if item.heading {
            let words = text(item.label.clone()).size(size::TEXT_SM.size).color(token::TEXT_2);
            let head = row().height(by_text(20.0)).padding_x(10.0).align_items(Align::Center).child(words);
            lines.push(Line::Part(Box::new(head)));
            height += 20.0;
            continue;
        }
        height += ROW;
        let mut line = menu::Row::new(face(&item, marks), item.label.clone());
        (line.line, line.disabled, line.checked, line.select) =
            (Some(div().padding_x(4.0)), item.disabled, item.checked, item.select.clone());
        if !item.items.is_empty() {
            // Beside its row, over it by 2 px.
            let inside = item.items.clone();
            line.inner = Some(Rc::new(move |at: &NodeRef, shown, ends| {
                let place = Place::Beside(at.clone(), Side::Right, Along::Start, -2.0);
                let inside = inside.clone();
                popup(shown, listing(place), Some(at.clone()), move || rows(inside.clone(), ends.clone()))
            }));
        }
        lines.push(Line::Row(Box::new(line)));
    }
    let list = menu::rows(column().min_width(200.0 - EDGE).padding_y(4.0), lines, ends);
    // A long menu shows so much of itself, and scrolls.
    match height > MOST {
        true => column().child(scroll(signal(0.0), list).container(div().height(MOST))),
        false => list,
    }
}
