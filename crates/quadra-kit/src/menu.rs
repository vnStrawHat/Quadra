//! The menu (specs/behaviour/15, "Menu"): rows that do something, in a popup. The arrows move
//! a highlight, Enter does what the highlighted row does, and doing it closes the menu.

use std::rc::Rc;

use quadra_layout::Align;
use quadra_reactive::{Signal, signal};
use quadra_tree::{Role, State};
use quadra_ui::{AnyView, Div, NodeRef, View, column, div, row, text};

use crate::action::{Confirm, MENU, MoveNext, MovePrevious};
use crate::icon::icon;
use crate::popup::popover;
use crate::{icons, size, token};

/// A row of a menu, or the line between two groups of rows.
#[derive(Clone, Default)]
pub struct MenuItem {
    label: String,
    icon: Option<&'static str>,
    keys: Option<String>,
    checked: Option<bool>,
    disabled: bool,
    separator: bool,
    select: Option<Rc<dyn Fn()>>,
}

impl MenuItem {
    pub fn new(label: impl Into<String>) -> MenuItem {
        MenuItem { label: label.into(), ..MenuItem::default() }
    }

    /// A hairline between two groups of rows.
    pub fn separator() -> MenuItem {
        MenuItem { separator: true, ..MenuItem::default() }
    }

    /// An icon before the label: one of [`crate::icons`].
    pub fn icon(mut self, body: &'static str) -> Self {
        self.icon = Some(body);
        self
    }

    /// The shortcut that does the same, as text after the label: "Ctrl-C".
    pub fn keys(mut self, keys: impl Into<String>) -> Self {
        self.keys = Some(keys.into());
        self
    }

    /// A row that is on or off: it shows a check while on.
    pub fn checked(mut self, on: bool) -> Self {
        self.checked = Some(on);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// What choosing the row does, before the menu closes.
    pub fn on_select(mut self, f: impl Fn() + 'static) -> Self {
        self.select = Some(Rc::new(f));
        self
    }

    fn chosen(&self) -> bool {
        !self.separator && !self.disabled
    }
}

/// The rows of a menu as a view. `done` runs after a row was chosen.
pub(crate) fn rows(items: Vec<MenuItem>, done: impl Fn() + Clone + 'static) -> Div {
    let items = Rc::new(items);
    // The row the keyboard is on, if any: the pointer has its own mark, the hover fill.
    let on: Signal<Option<usize>> = signal(None);
    let marks = items.iter().any(|item| item.icon.is_some() || item.checked.is_some());
    let mut parts: Vec<AnyView> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        if item.separator {
            parts
                .push(AnyView::new(div().height(13.0).padding_y(6.0).child(div().grow(1.0).bg(token::LINE))));
            continue;
        }
        // 23 px high, 4 px in from the menu's edge, 6 px beside what it holds.
        let face = row().height(23.0).grow(1.0).padding_x(6.0).gap(8.0).radius(size::RADIUS_MD);
        let face = face.align_items(Align::Center).role(Role::MenuItem).label(item.label.clone());
        let face = face.hover(|s| s.bg(token::PANEL_HOVER)).selected(move || on.get() == Some(i));
        let face = face.when(State::SELECTED, |s| s.bg(token::PRESS));
        let face = face.disabled(item.disabled).when(State::DISABLED, |s| s.text_color(token::TEXT_DISABLED));
        let face = match item.checked {
            Some(checked) => face.checked(checked),
            None => face,
        };
        let face = match (marks, item.checked, item.icon) {
            (false, ..) => face,
            (_, Some(true), _) => face.child(icon(icons::CHECK).color(token::ACCENT_TEXT)),
            (_, None, Some(body)) => face.child(icon(body).color(token::TEXT_2)),
            _ => face.child(div().width(14.0)),
        };
        let face = face.child(div().grow(1.0).child(text(item.label.clone())));
        let face = match &item.keys {
            Some(keys) => face.child(div().padding_x(8.0).child(text(keys.clone()).color(token::TEXT_2))),
            None => face,
        };
        let (select, done) = (item.select.clone(), done.clone());
        let face = face.on_click(move |_| {
            if let Some(select) = &select {
                select();
            }
            done();
        });
        parts.push(AnyView::new(div().padding_x(4.0).child(face)));
    }
    // The arrows go to the next row that can be chosen, round the ends.
    let step = {
        let items = items.clone();
        move |forward: bool| {
            let count = items.len();
            let from = on.get_untracked().unwrap_or(if forward { count - 1 } else { 0 });
            let order =
                (1..=count).map(|k| if forward { (from + k) % count } else { (from + count - k) % count });
            on.set({ order }.find(|&i| items[i].chosen()));
        }
    };
    let (forward, back) = (step.clone(), step);
    let confirm = move || {
        let Some(item) = on.get_untracked().and_then(|i| items.get(i)) else { return };
        if let Some(select) = &item.select {
            select();
        }
        done();
    };
    let list = column().role(Role::Menu).min_width(200.0).padding_y(4.0).focusable().key_context(MENU);
    let list = list.on_action::<MoveNext>(move |_| forward(true));
    let list = list.on_action::<MovePrevious>(move |_| back(false));
    list.on_action::<Confirm>(move |_| confirm()).children(parts)
}

/// A menu below `opener` while `open` is true. `items` is asked each time the menu opens,
/// so a row can say what is so then (a check, a disabled row).
pub fn menu<F>(open: Signal<bool>, opener: &NodeRef, items: F) -> impl View + use<F>
where
    F: Fn() -> Vec<MenuItem> + 'static,
{
    popover(open, opener, move || rows(items(), move || open.set(false)))
}
