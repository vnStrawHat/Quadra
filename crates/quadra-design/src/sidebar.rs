//! The sidebar (specs/behaviour/09, "Navigation"): the places of an app in groups, one of
//! them chosen, a search that keeps those it names. It folds to its icons alone.

use quadra_headless::list;
use quadra_layout::{Align, Display, Justify, Length};
use quadra_platform::Cursor;
use quadra_reactive::{Signal, memo, signal};
use quadra_tree::{NodeId, Paint, Side, State};
use std::rc::Rc;

use quadra_view::{AnyView, Div, MountCx, View, by_text, column, div, row, scroll, text};

use crate::display::{chosen_mark, label};
use crate::icon::icon;
use crate::{fonts, icons, input, size, token};

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
type Counts = Rc<dyn Fn(&'static str) -> Option<usize>>;

pub struct Sidebar {
    groups: Vec<(String, Vec<NavItem>)>,
    chosen: Signal<&'static str>,
    folded: Folded,
    width: f32,
    head: Option<AnyView>,
    foot: Option<AnyView>,
    query: Option<Signal<String>>,
    counts: Option<Counts>,
}

/// A sidebar with no places yet: `chosen` holds the id of the chosen one, and a click or the
/// arrows write it.
pub fn sidebar(chosen: Signal<&'static str>) -> Sidebar {
    let folded = Box::new(|| false);
    Sidebar {
        groups: Vec::new(),
        chosen,
        folded,
        width: 240.0,
        head: None,
        foot: None,
        query: None,
        counts: None,
    }
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

    /// A search field at the top: the places whose names hold `query`, any case, are kept.
    pub fn search(mut self, query: Signal<String>) -> Self {
        self.query = Some(query);
        self
    }

    /// A strip above the places that does not scroll: the name of the area and a small
    /// button at its end (Refresh). 28 px, 12 px muted text, over a line.
    pub fn head(mut self, view: impl View + 'static) -> Self {
        self.head = Some(AnyView::new(view));
        self
    }

    /// Something under the places, outside their scroll, over a line: a filter box.
    pub fn foot(mut self, view: impl View + 'static) -> Self {
        self.foot = Some(AnyView::new(view));
        self
    }

    /// A count at the end of each place `counts` gives one for, by the place's id: live as
    /// what it reads changes.
    pub fn counts(mut self, counts: impl Fn(&'static str) -> Option<usize> + 'static) -> Self {
        self.counts = Some(Rc::new(counts));
        self
    }
}

impl View for Sidebar {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (chosen, width, folded) = (self.chosen, self.width, memo(self.folded));
        // What is words is not there while the sidebar is folded.
        let words = move || if folded.get() { Display::None } else { Display::Flex };
        // The names the query keeps; all of them without a query.
        let query = self.query;
        let names: Vec<(&'static str, String)> = (self.groups.iter())
            .flat_map(|(_, items)| items.iter().map(|i| (i.id, i.label.to_lowercase())))
            .collect();
        let kept = memo(move || {
            let asked = query.map(|q| q.with(|q| q.trim().to_lowercase())).unwrap_or_default();
            names.iter().filter(|(_, name)| name.contains(&asked)).map(|(id, _)| *id).collect::<Vec<_>>()
        });
        let shown = move |yes: bool| if yes && !folded.get() { Display::Flex } else { Display::None };
        let mut places = column().padding_y(token::SPACE_XS);
        if let Some(query) = query {
            places = places.child(search(query, kept, chosen).display(move || shown(true)));
        }
        for (title, items) in self.groups {
            let of: Vec<&'static str> = items.iter().map(|i| i.id).collect();
            if !title.is_empty() {
                let title = div()
                    .padding_left(token::SPACE_LG)
                    .padding_top(token::SPACE_LG)
                    .padding_bottom(token::SPACE_XS)
                    .child(label(title));
                let any = move || kept.with(|kept| of.iter().any(|id| kept.contains(id)));
                places = places.child(title.display(move || shown(any())));
            }
            for item in items {
                let id = item.id;
                let row = place(item, chosen, words, move || folded.get(), self.counts.clone());
                places = places.child(row.display(move || match kept.with(|kept| kept.contains(&id)) {
                    true => Display::Flex,
                    false => Display::None,
                }));
            }
        }
        let none = div()
            .padding_x(token::SPACE_LG)
            .padding_y(token::SPACE_MD)
            .text_size(size::TEXT_SM.size)
            .text_color(token::TEXT_3);
        let none = none.display(move || shown(kept.with(Vec::is_empty))).child(text("No results"));
        places = places.child(none);
        let side = column().shrink(0.0).width(move || Length::Px(if folded.get() { 40.0 } else { width }));
        let side = side.bg(token::PANEL).border_right(1.0).border_color(token::LINE_STRONG);
        let side = side.focus_visible(|s| s.outline(token::BORDER_FOCUSED));
        let places = column().grow(1.0).min_height(0.0).child(scroll(signal(0.0), places));
        // The arrows go among the places the search keeps.
        let side = list::places(side, move || kept.get(), chosen);
        let side = match self.head {
            Some(view) => {
                let strip = row().align_items(Align::Center).shrink(0.0).height(by_text(28.0));
                let strip = strip
                    .padding_left(token::SPACE_LG)
                    .padding_right(token::SPACE_XS)
                    .border_bottom(1.0)
                    .border_color(token::LINE);
                let strip = strip.text_size(size::TEXT_SM.size).text_color(token::TEXT_2);
                side.child(
                    strip
                        .display(move || shown(true))
                        .child(column().grow(1.0).align_items(Align::Stretch).child(view)),
                )
            }
            None => side,
        };
        let side = side.child(places);
        let side = match self.foot {
            Some(view) => {
                let foot =
                    row().shrink(0.0).padding(token::SPACE_MD).border_top(1.0).border_color(token::LINE);
                side.child(foot.display(move || shown(true)).child(view))
            }
            None => side,
        };
        side.mount(cx)
    }
}

/// The row of a place: 26 px, 12 px in from each side, its icon of 14 px, its label and its
/// count, 6 px apart; filled when chosen, with
/// the accent's mark at its left and its icon in the accent.
fn place(
    item: NavItem,
    chosen: Signal<&'static str>,
    words: impl Fn() -> Display + Copy + 'static,
    folded: impl Fn() -> bool + 'static,
    counts: Option<Counts>,
) -> Div {
    let id = item.id;
    let line = row()
        .align_items(Align::Center)
        .gap(token::SPACE_SM)
        .height(by_text(28.0))
        .padding_x(token::SPACE_LG);
    let line = line.justify(move || if folded() { Justify::Center } else { Justify::Start });
    let (name, count) = (item.label.clone(), counts.clone());
    let line = line.cursor(Cursor::Pointer).label(move || match count.as_ref().and_then(|of| of(id)) {
        Some(n) => format!("{name}, {n}"),
        None => name.clone(),
    });
    let on = move || chosen.get() == id;
    let line = list::place(line, id, chosen);
    let line = line.hover(|s| s.bg(token::RAISED_HOVER)).when(State::SELECTED, |s| s.bg(token::ACCENT_TINT));
    let tone = move || Paint::from(if on() { token::ACCENT_TEXT } else { token::TEXT_2 });
    let line = match item.icon {
        Some(body) => line.child(div().text_color(tone).child(icon(body).size(14.0))),
        None => line,
    };
    let line = line.child(div().grow(1.0).min_width(0.0).display(words).child(text(item.label)));
    let line = match counts {
        Some(of) => {
            let (said, there) = (of.clone(), of);
            let count =
                div().font_family(fonts::MONO).text_size(size::TEXT_SM.size).text_color(token::TEXT_3);
            let count = count.display(move || match there(id) {
                Some(_) => words(),
                None => Display::None,
            });
            line.child(count.child(text(move || said(id).map(|n| n.to_string()).unwrap_or_default())))
        }
        None => line,
    };
    line.child(chosen_mark(Side::Left, on))
}

/// The field at the top: Enter chooses the first place kept.
fn search(
    query: Signal<String>,
    kept: quadra_reactive::Memo<Vec<&'static str>>,
    chosen: Signal<&'static str>,
) -> Div {
    let field =
        input(query).small().height(28.0).fill().icon(icons::SEARCH).placeholder("Search…").label("Search");
    let field = field.on_submit(move |_| {
        if let Some(first) = quadra_reactive::untrack(|| kept.with(|kept| kept.first().copied())) {
            chosen.set(first);
        }
    });
    row().padding_x(token::SPACE_MD).padding_top(token::SPACE_XS).padding_bottom(token::SPACE_XS).child(field)
}
