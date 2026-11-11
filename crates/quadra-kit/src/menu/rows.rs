//! The rows of a menu (specs/behaviour/15, "Menu"): what each looks like, the highlight the
//! arrows move, and the menu a row opens beside itself.

use std::rc::Rc;

use quadra_layout::Align;
use quadra_reactive::{Signal, signal};
use quadra_tree::{Along, PointerKind, Role, Side, State};
use quadra_ui::{AnyView, Div, NodeRef, Place, column, div, row, scroll, text};

use super::MenuItem;
use crate::action::{Confirm, MENU, MoveInto, MoveNext, MoveOut, MovePrevious};
use crate::icon::icon;
use crate::popup::{EDGE, Shown, popup};
use crate::{icons, size, token};

/// The height of a row, and the most of a menu that shows before it scrolls.
const ROW: f32 = 23.0;
const MOST: f32 = 450.0;

/// What ends a menu: `done` after a row was chosen, and `leave` for the menu of a row, which
/// the left arrow gives up.
#[derive(Clone)]
pub(crate) struct Ends {
    pub done: Rc<dyn Fn()>,
    pub leave: Option<Rc<dyn Fn()>>,
}

/// A row that is chosen: 23 px high, 4 px in from the menu's edge, 6 px beside what it holds
/// and between its parts, the shortcut at the right and 16 px from the label at least.
fn face(item: &MenuItem, marks: bool) -> Div {
    let face = row().height(ROW).grow(1.0).padding_x(6.0).gap(6.0).radius(size::RADIUS_MD);
    let face = face.align_items(Align::Center).role(Role::MenuItem).label(item.label.clone());
    let face = face.hover(|s| s.bg(token::PANEL_HOVER)).when(State::SELECTED, |s| s.bg(token::PRESS));
    let face = if item.danger { face.text_color(token::ERROR) } else { face };
    let face = face.disabled(item.disabled).when(State::DISABLED, |s| s.text_color(token::TEXT_DISABLED));
    let face = match item.checked {
        Some(checked) => face.checked(checked),
        None => face,
    };
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

/// The rows of a menu as a view.
pub(crate) fn rows(items: Vec<MenuItem>, ends: Ends) -> Div {
    let items = Rc::new(items);
    // The row the keyboard is on, if any (the pointer has its own mark, the hover fill), and
    // the row whose menu is open beside it.
    let on: Signal<Option<usize>> = signal(None);
    let beside: Signal<Option<usize>> = signal(None);
    let marks = items.iter().any(|item| item.icon.is_some() || item.checked.is_some());
    let refs: Vec<NodeRef> = items.iter().map(|_| NodeRef::new()).collect();
    let mut parts: Vec<AnyView> = Vec::new();
    let mut height = 8.0;
    for (i, item) in items.iter().enumerate() {
        if item.separator {
            let line = div().height(13.0).padding_y(6.0).child(div().grow(1.0).bg(token::LINE));
            parts.push(AnyView::new(line));
            height += 13.0;
            continue;
        }
        if item.heading {
            let words = text(item.label.clone()).size(size::TEXT_SM.size).color(token::TEXT_2);
            let head = row().height(20.0).padding_x(10.0).align_items(Align::Center).child(words);
            parts.push(AnyView::new(head));
            height += 20.0;
            continue;
        }
        height += ROW;
        let face = face(item, marks).selected(move || on.get() == Some(i)).node_ref(&refs[i]);
        let inner = !item.items.is_empty();
        // The pointer over a row opens its menu, and closes that of any other row.
        let face = face.on_pointer(PointerKind::Enter, move |_| beside.set(inner.then_some(i)));
        let (select, done) = (item.select.clone(), ends.done.clone());
        let face = face.on_click(move |_| {
            if inner {
                return beside.set(Some(i));
            }
            if let Some(select) = &select {
                select();
            }
            done();
        });
        let line = div().padding_x(4.0).child(face);
        parts.push(AnyView::new(match inner {
            false => line,
            true => {
                // Beside its row, over it by 2 px; a choice in it ends the menus together.
                let shown = Shown {
                    is: Rc::new(move || beside.get() == Some(i)),
                    close: Rc::new(move || beside.update(|open| *open = open.filter(|open| *open != i))),
                };
                let place = Place::Beside(refs[i].clone(), Side::Right, Along::Start, -2.0);
                let (inside, done) = (item.items.clone(), ends.done.clone());
                let leave: Rc<dyn Fn()> = Rc::new(move || beside.set(None));
                let content =
                    move || rows(inside.clone(), Ends { done: done.clone(), leave: Some(leave.clone()) });
                line.child(popup(shown, (place, 1), Some(refs[i].clone()), content))
            }
        }));
    }
    // The arrows go to the next row that can be chosen, round the ends, and bring it into view.
    let step = {
        let (items, refs) = (items.clone(), refs);
        move |forward: bool| {
            let count = items.len();
            let from = on.get_untracked().unwrap_or(if forward { count - 1 } else { 0 });
            let order =
                (1..=count).map(|k| if forward { (from + k) % count } else { (from + count - k) % count });
            on.set({ order }.find(|&i| items[i].chosen()));
            if let Some(at) = on.get_untracked() {
                refs[at].scroll_into_view();
            }
        }
    };
    let (forward, back, opens, leave) = (step.clone(), step, items.clone(), ends.leave);
    let confirm = move || {
        let Some((at, item)) = on.get_untracked().and_then(|i| Some((i, items.get(i)?))) else { return };
        if !item.items.is_empty() {
            return beside.set(Some(at));
        }
        if let Some(select) = &item.select {
            select();
        }
        (ends.done)();
    };
    let list = column().role(Role::Menu).min_width(200.0 - EDGE).padding_y(4.0).focusable().key_context(MENU);
    let list = list.on_action::<MoveNext>(move |_| forward(true));
    let list = list.on_action::<MovePrevious>(move |_| back(false));
    let list = list.on_action::<Confirm>(move |_| confirm());
    // The right arrow opens the menu of the row it is on; the left one gives this menu up.
    let list = list.on_action::<MoveInto>(move |_| {
        let inner = on.get_untracked().filter(|&i| !opens[i].items.is_empty());
        if inner.is_some() {
            beside.set(inner);
        }
    });
    let list = list.on_action::<MoveOut>(move |_| {
        if let Some(leave) = &leave {
            leave();
        }
    });
    let list = list.children(parts);
    // A long menu shows so much of itself, and scrolls.
    match height > MOST {
        true => column().child(scroll(signal(0.0), list).container(div().height(MOST))),
        false => list,
    }
}
