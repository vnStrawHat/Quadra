//! The rows of a menu (specs/behaviour/15, "Menu"): what each looks like, and where the menu
//! a row opens stands.

use std::rc::Rc;

use quadra_base::menu::{self, Ends, Line};
use quadra_layout::Align;
use quadra_reactive::signal;
use quadra_tree::{Along, Side, State};
use quadra_ui::{Div, NodeRef, Place, column, div, row, scroll, text};

use super::MenuItem;
use crate::icon::icon;
use crate::popup::{EDGE, popup};
use crate::{icons, size, token};

/// The height of a row, and the most of a menu that shows before it scrolls.
const ROW: f32 = 23.0;
const MOST: f32 = 450.0;

/// A row that is chosen: 23 px high, 4 px in from the menu's edge, 6 px beside what it holds
/// and between its parts, the shortcut at the right and 16 px from the label at least.
fn face(item: &MenuItem, marks: bool) -> Div {
    let face = row().height(ROW).grow(1.0).padding_x(6.0).gap(6.0).radius(token::RADIUS_MD);
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
        _ => face.child(div().width(14.0)),
    };
    let face = face.child(text(item.label.clone())).child(div().grow(1.0).min_width(16.0));
    let face = match &item.keys {
        Some(keys) if item.disabled => face.child(text(keys.clone())),
        Some(keys) => face.child(text(keys.clone()).color(token::TEXT_2)),
        None => face,
    };
    match item.items.is_empty() {
        true => face,
        false => face.child(icon(icons::CHEVRON_RIGHT).size(12.0).color(token::TEXT_2)),
    }
}

/// The rows of a menu as a view: the base's (`quadra_base::menu`) with the kit's faces.
pub(crate) fn rows(items: Vec<MenuItem>, ends: Ends) -> Div {
    let marks = items.iter().any(|item| item.icon.is_some() || item.checked.is_some());
    let mut lines = Vec::new();
    let mut height = 8.0;
    for item in items {
        if item.separator {
            let rule = div().height(13.0).padding_y(6.0).child(div().grow(1.0).bg(token::LINE));
            lines.push(Line::Part(Box::new(rule)));
            height += 13.0;
            continue;
        }
        if item.heading {
            let words = text(item.label.clone()).size(size::TEXT_SM.size).color(token::TEXT_2);
            let head = row().height(20.0).padding_x(10.0).align_items(Align::Center).child(words);
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
                popup(shown, (place, 1), Some(at.clone()), move || rows(inside.clone(), ends.clone()))
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
