//! Settings (specs/13, "Components"; behaviour/15, "Settings"): the preferences of an app in
//! pages. A column of the pages at the left; the chosen page at the right, a row for each
//! setting: what it is called and a line about it, then its control. The app owns the
//! controls, the text searched for and the chosen page; the component keeps which pages are
//! open in the column, the chosen group and where the page is scrolled.

mod cards;
mod rules;

use quadra_headless::settings as base;
use quadra_headless::settings::Model;
use quadra_platform::Cursor;
use quadra_reactive::{Signal, memo, signal};
use quadra_tree::NodeId;
use quadra_ui::{AnyView, Div, MountCx, View, row};

use crate::input::{Input, input};
use crate::{icons, size, token};

/// A setting: what it is called, a line about it, and its control.
struct Setting {
    title: String,
    about: String,
    control: AnyView,
}

impl Setting {
    /// What a search looks in: its title and the line about it.
    fn text(&self) -> String {
        format!("{}\n{}", self.title, self.about)
    }
}

/// Settings that belong together, under a title.
pub struct SettingsGroup {
    title: String,
    about: String,
    rows: Vec<Setting>,
}

impl SettingsGroup {
    /// A group with no settings yet; an empty title is no title.
    pub fn new(title: impl Into<String>) -> SettingsGroup {
        SettingsGroup { title: title.into(), about: String::new(), rows: Vec::new() }
    }

    /// A line under the title.
    pub fn about(mut self, words: impl Into<String>) -> Self {
        self.about = words.into();
        self
    }

    /// A setting: its title, a line about it (empty for none), and its control at the right.
    pub fn row(mut self, title: &str, about: &str, control: impl View + 'static) -> Self {
        let (title, about) = (title.to_string(), about.to_string());
        self.rows.push(Setting { title, about, control: AnyView::new(control) });
        self
    }

    /// A row that is its control alone, as wide as the page. A search never keeps it.
    pub fn wide(self, control: impl View + 'static) -> Self {
        self.row("", "", control)
    }
}

/// A group of settings on its own, in any page: its title over an outlined card of its rows,
/// each a title and a line about it at the left and its control at the right (behaviour/15,
/// "Settings, cards"). The groups of `settings(..).cards()` look so.
pub fn settings_group(group: SettingsGroup) -> Div {
    let always = memo(|| true);
    cards::group_card(group, always, move |_| always)
}

/// A page of settings: its groups, under a title.
pub struct SettingsPage {
    title: String,
    icon: Option<&'static str>,
    about: String,
    groups: Vec<SettingsGroup>,
}

impl SettingsPage {
    pub fn new(title: impl Into<String>) -> SettingsPage {
        SettingsPage { title: title.into(), icon: None, about: String::new(), groups: Vec::new() }
    }

    /// An icon before the title in the column (cards): one of [`crate::icons`].
    pub fn icon(mut self, body: &'static str) -> Self {
        self.icon = Some(body);
        self
    }

    /// A line under the page's title (cards).
    pub fn about(mut self, words: impl Into<String>) -> Self {
        self.about = words.into();
        self
    }

    /// Adds a group to the page, shown by [`settings`] with its search and pages. A group shown
    /// alone, outside of them, is [`settings_group`].
    pub fn group(mut self, group: SettingsGroup) -> Self {
        self.groups.push(group);
        self
    }
}

pub struct Settings {
    pages: Vec<SettingsPage>,
    current: Signal<usize>,
    query: Option<Signal<String>>,
    header: Vec<AnyView>,
    hint: Option<(String, String)>,
    cards: bool,
}

/// Settings with no pages yet: `current` holds the index of the chosen page, and a click in
/// the column writes it.
pub fn settings(current: Signal<usize>) -> Settings {
    Settings { pages: Vec::new(), current, query: None, header: Vec::new(), hint: None, cards: false }
}

impl Settings {
    pub fn page(mut self, page: SettingsPage) -> Self {
        self.pages.push(page);
        self
    }

    /// A search field above the column, writing `query`: while it holds text, only the rows
    /// show whose title or line about them holds it, whatever the case and the accents.
    pub fn search(mut self, query: Signal<String>) -> Self {
        self.query = Some(query);
        self
    }

    /// Something beside the page: a row above its title, or the right end of its header
    /// (cards). Scope toggles, a button that opens the file.
    pub fn header(mut self, view: impl View + 'static) -> Self {
        self.header.push(AnyView::new(view));
        self
    }

    /// A key hint under the column (rules).
    pub fn hint(mut self, keys: &str, does: &str) -> Self {
        self.hint = Some((keys.to_string(), does.to_string()));
        self
    }

    /// The layout of outlined cards (behaviour/15, "Settings, cards").
    pub fn cards(mut self) -> Self {
        self.cards = true;
        self
    }
}

/// A row of the column does `f` on a click, and on Enter and Space with the focus, which
/// shows as a line around it.
fn acts(line: Div, f: impl Fn() + Clone + 'static) -> Div {
    let line = line.cursor(Cursor::Pointer).focus_visible(|s| s.outline(token::BORDER_FOCUSED));
    base::acts(line, f)
}

/// The search field of the column, `width` wide.
fn search(query: Signal<String>, width: f32) -> Input {
    input(query).placeholder("Search settings…").icon(icons::SEARCH).width(width)
}

impl View for Settings {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let texts: Vec<Vec<Vec<String>>> = (self.pages.iter())
            .map(|page| {
                page.groups.iter().map(|group| group.rows.iter().map(Setting::text).collect()).collect()
            })
            .collect();
        let query = self.query.unwrap_or_else(|| signal(String::new()));
        let model = Model::new(&texts, self.current, query);
        let whole = row().grow(1.0).min_height(0.0).bg(token::CANVAS).text_color(token::TEXT);
        let whole = whole.text_size(size::TEXT_MD.size);
        let whole =
            match self.cards {
                true => whole.child(cards::nav(&self.pages, &model, self.query)).child(cards::page(
                    self.pages,
                    &model,
                    self.header,
                )),
                false => whole
                    .child(rules::nav(&self.pages, &model, self.query, self.hint))
                    .child(rules::page(self.pages, &model, self.header)),
            };
        whole.mount(cx)
    }
}
