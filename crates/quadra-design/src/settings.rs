//! Settings (specs/13, "Components"; behaviour/15, "Settings"): the preferences of an app in
//! pages. A column of the pages at the left; the chosen page at the right, a row for each
//! setting: what it is called and a line about it, then its control. The app owns the
//! controls, the text searched for and the chosen page; the component keeps which pages are
//! open in the column, the chosen group and where the page is scrolled.

mod cards;
mod rules;

use quadra_headless::settings as base;
use quadra_headless::settings::{Model, Reset, shown};
use quadra_layout::Align;
use quadra_platform::Cursor;
use quadra_reactive::{Signal, memo, signal};
use quadra_tree::{NodeId, Role, Tree};
use quadra_view::{AnyView, Div, MountCx, View, div, row};

use crate::button::{Size, Variant, button, icon_button};
use crate::input::{Input, input};
use crate::{icons, size, token};

/// A setting: what it is called, a line about it, its control, and how it goes back to its
/// default.
struct Setting {
    title: String,
    about: String,
    control: AnyView,
    reset: Option<Reset>,
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
        self.rows.push(Setting { title, about, control: AnyView::new(control), reset: None });
        self
    }

    /// The last row goes back to `default`: while `value` differs from it, a button left of
    /// the control puts it back, and the page's "Reset all" counts the row.
    pub fn resets<T: PartialEq + Clone + 'static>(self, value: Signal<T>, default: T) -> Self {
        self.reset_by(Reset::of(value, default))
    }

    /// The last row differs from its default while `changed` holds, and `reset` puts it
    /// back: for a row of more than one value.
    pub fn resets_with(self, changed: impl Fn() -> bool + 'static, reset: impl Fn() + 'static) -> Self {
        self.reset_by(Reset::new(changed, reset))
    }

    fn reset_by(mut self, reset: Reset) -> Self {
        if let Some(last) = self.rows.last_mut() {
            last.reset = Some(reset);
        }
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

/// The control of a row at the right, after the button that resets it while it differs; the
/// row's title names it for a reader.
fn control(title: &str, control: AnyView, reset: Option<Reset>) -> Div {
    let name = title.to_string();
    let at = div().shrink(0.0).on_mount(move |tree, id| _ = name_control(tree, id, &name));
    let Some(reset) = reset else { return at.child(control) };
    let changed = reset.changed;
    let back = icon_button(icons::UNDO, "").label(format!("Reset {title}"));
    let back = div().display(move || shown(changed.get())).child(back.on_click(move || reset.reset()));
    at.align_items(Align::Center).gap(4.0).child(back).child(control)
}

/// Names the first switch, checkbox, input or slider under `id` that has no name: a reader
/// hears what the row is called, not "switch".
fn name_control(tree: &mut Tree, id: NodeId, name: &str) -> bool {
    let says = tree.meaning(id);
    if matches!(says.role, Role::Switch | Role::Checkbox | Role::TextInput | Role::Slider) {
        if says.name.is_empty() && !(says.role.named_by_content() && has_text(tree, id)) {
            tree.update_semantics(id, |it| it.name = name.to_string());
        }
        return true;
    }
    tree.children(id).to_vec().into_iter().any(|child| name_control(tree, child, name))
}

/// Whether a text says something inside `id`: a switch with words of its own is named by them.
fn has_text(tree: &Tree, id: NodeId) -> bool {
    tree.children(id).iter().any(|&child| {
        let says = tree.meaning(child);
        (says.role == Role::Text && !says.name.trim().is_empty()) || has_text(tree, child)
    })
}

/// A page's "Reset all", shown while one of the rows a search keeps differs; none for a
/// page of no row that resets.
fn reset_all(page: &SettingsPage, at: usize, model: &Model) -> Option<Div> {
    let rows = (page.groups.iter().enumerate()).flat_map(|(g, group)| {
        let kept = &model.rows[at][g];
        group.rows.iter().zip(kept).filter_map(|(row, kept)| Some((*kept, row.reset.clone()?)))
    });
    let rows: Vec<_> = rows.collect();
    if rows.is_empty() {
        return None;
    }
    let (any, all) = base::reset_all(rows);
    let on = model.shown;
    let all = button("Reset all").icon(icons::UNDO).variant(Variant::Ghost).size(Size::Sm).on_click(all);
    Some(div().display(move || shown(on.get() == at && any.get())).child(all))
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
        let resets: Vec<_> =
            self.pages.iter().enumerate().map(|(i, page)| reset_all(page, i, &model)).collect();
        let whole = row().grow(1.0).min_height(0.0).bg(token::CANVAS).text_color(token::TEXT);
        let whole = whole.text_size(size::TEXT_MD.size);
        let whole =
            match self.cards {
                true => whole.child(cards::nav(&self.pages, &model, self.query)).child(cards::page(
                    self.pages,
                    &model,
                    resets.into_iter().flatten().map(AnyView::new).chain(self.header).collect(),
                )),
                false => whole
                    .child(rules::nav(&self.pages, &model, self.query, self.hint))
                    .child(rules::page(self.pages, &model, resets, self.header)),
            };
        whole.mount(cx)
    }
}
