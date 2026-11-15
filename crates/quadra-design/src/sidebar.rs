//! The sidebar (specs/behaviour/09, "Navigation"): the places of an app in groups, one of
//! them chosen. It folds to its icons alone.

use quadra_headless::list;
use quadra_layout::{Align, Display, Justify, Length};
use quadra_platform::Cursor;
use quadra_reactive::{Signal, memo, signal};
use quadra_tree::{NodeId, Paint, Side, State};
use quadra_ui::{AnyView, Div, MountCx, View, by_text, column, div, row, scroll, text};

use crate::display::{chosen_mark, label};
use crate::icon::icon;
use crate::token;

/// A place: what it is known by, what it is called, and its icon if it has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavItem {
    pub id: &'static str,
    pub label: String,
    pub icon: Option<&'static str>,
}

impl NavItem {
    pub fn new(id: &'static str, label: impl Into<String>) -> NavItem {
        NavItem { id, label: label.into(), icon: None }
    }

    pub fn icon(mut self, body: &'static str) -> NavItem {
        self.icon = Some(body);
        self
    }
}

type Folded = Box<dyn Fn() -> bool>;

pub struct Sidebar {
    groups: Vec<(String, Vec<NavItem>)>,
    chosen: Signal<&'static str>,
    folded: Folded,
    width: f32,
    head: Option<AnyView>,
}

/// A sidebar with no places yet: `chosen` holds the id of the chosen one, and a click or the
/// arrows write it.
pub fn sidebar(chosen: Signal<&'static str>) -> Sidebar {
    Sidebar { groups: Vec::new(), chosen, folded: Box::new(|| false), width: 240.0, head: None }
}

impl Sidebar {
    /// A group of places under a title; an empty title is no title.
    pub fn group(mut self, title: impl Into<String>, items: impl IntoIterator<Item = NavItem>) -> Self {
        self.groups.push((title.into(), items.into_iter().collect()));
        self
    }

    /// While `folded` is true the sidebar is 40 px wide and shows the icons alone.
    pub fn folded(mut self, folded: impl Fn() -> bool + 'static) -> Self {
        self.folded = Box::new(folded);
        self
    }

    /// Its width while not folded: 240 px unless told.
    pub fn width(mut self, px: f32) -> Self {
        self.width = px;
        self
    }

    /// Something above the places: a search field, the name of a project.
    pub fn head(mut self, view: impl View + 'static) -> Self {
        self.head = Some(AnyView::new(view));
        self
    }
}

impl View for Sidebar {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (chosen, width, folded) = (self.chosen, self.width, memo(self.folded));
        // What is words is not there while the sidebar is folded.
        let words = move || if folded.get() { Display::None } else { Display::Flex };
        let ids: Vec<&'static str> =
            self.groups.iter().flat_map(|(_, items)| items.iter().map(|i| i.id)).collect();
        let mut places = column().padding_y(4.0);
        if let Some(head) = self.head {
            places = places.child(head);
        }
        for (title, items) in self.groups {
            if !title.is_empty() {
                let title =
                    div().padding_left(10.0).padding_top(12.0).padding_bottom(4.0).child(label(title));
                places = places.child(title.display(words));
            }
            for item in items {
                places = places.child(place(item, chosen, words, move || folded.get()));
            }
        }
        let side = column().shrink(0.0).width(move || Length::Px(if folded.get() { 40.0 } else { width }));
        let side = side.bg(token::PANEL).border_right(1.0).border_color(token::LINE_STRONG);
        let side = side.focus_visible(|s| s.outline(token::BORDER_FOCUSED));
        list::places(side, ids, chosen).child(scroll(signal(0.0), places)).mount(cx)
    }
}

/// The row of a place: 26 px, its icon and its label; filled when chosen, with the accent's
/// mark at its left and its icon in the accent.
fn place(
    item: NavItem,
    chosen: Signal<&'static str>,
    words: impl Fn() -> Display + 'static,
    folded: impl Fn() -> bool + 'static,
) -> Div {
    let id = item.id;
    let line =
        row().align_items(Align::Center).gap(6.0).height(by_text(26.0)).padding_left(10.0).padding_right(8.0);
    let line = line.justify(move || if folded() { Justify::Center } else { Justify::Start });
    let line = line.cursor(Cursor::Pointer).label(item.label.clone());
    let on = move || chosen.get() == id;
    let line = list::place(line, id, chosen);
    let line = line.hover(|s| s.bg(token::RAISED_HOVER)).when(State::SELECTED, |s| s.bg(token::PRESS));
    let tone = move || Paint::from(if on() { token::ACCENT_TEXT } else { token::TEXT_2 });
    let line = match item.icon {
        Some(body) => line.child(div().text_color(tone).child(icon(body).size(16.0))),
        None => line,
    };
    line.child(div().display(words).child(text(item.label))).child(chosen_mark(Side::Left, on))
}
