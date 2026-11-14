//! Settings (specs/13, "Components"; behaviour/15, "Settings"): the preferences of an app in
//! pages. A column of the pages at the left; the chosen page at the right, a row for each
//! setting: what it is called and a line about it, then its control. The app owns the
//! controls, the text searched for and the chosen page; the component keeps which pages are
//! open in the column, the chosen group and where the page is scrolled.

mod cards;
mod rules;

use std::cell::OnceCell;
use std::rc::Rc;

use quadra_layout::Display;
use quadra_platform::Cursor;
use quadra_reactive::{Memo, Signal, memo, signal};
use quadra_text::fold_diacritics;
use quadra_tree::{Handle, NodeId, SemanticAction};
use quadra_ui::{AnyView, Div, MountCx, NodeRef, View, row};

use crate::action::{CONTROL, Confirm};
use crate::input::{Input, input};
use crate::{icons, size, token};

/// A setting: what it is called, a line about it, and its control.
struct Setting {
    title: String,
    about: String,
    control: AnyView,
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

/// What the column and the page share: what a search keeps, and what the component keeps.
#[derive(Clone)]
struct Model {
    current: Signal<usize>,
    searching: Memo<bool>,
    /// Whether a row is kept, by page, group and row; whether a group and a page keep one.
    rows: Rc<Vec<Vec<Vec<Memo<bool>>>>>,
    groups: Rc<Vec<Vec<Memo<bool>>>>,
    pages: Rc<Vec<Memo<bool>>>,
    /// The page at the right: the chosen one, or while a search leaves it nothing the first
    /// page that keeps a row.
    shown: Memo<usize>,
    /// The pages whose groups show under them in the column.
    open: Rc<Vec<Signal<bool>>>,
    /// The group chosen in the column: its page and its place there.
    picked: Signal<Option<(usize, usize)>>,
    scroll: Signal<f32>,
    /// What a chosen group is scrolled by: the header of each, and what the page scrolls.
    heads: Rc<Vec<Vec<NodeRef>>>,
    content: Rc<OnceCell<(Handle, NodeId)>>,
}

impl Model {
    fn new(pages: &[SettingsPage], current: Signal<usize>, query: Signal<String>) -> Model {
        let plain = |text: &str| fold_diacritics(text).to_lowercase();
        let needle = memo(move || query.with(|query| plain(query)));
        let hit = |row: &Setting| {
            let hay = plain(&format!("{}\n{}", row.title, row.about));
            memo(move || needle.with(|needle| needle.is_empty() || hay.contains(needle)))
        };
        let any = |all: Vec<Memo<bool>>| memo(move || all.iter().any(|kept| kept.get()));
        let per_group = |page: &SettingsPage| -> Vec<Vec<Memo<bool>>> {
            page.groups.iter().map(|group| group.rows.iter().map(hit).collect()).collect()
        };
        let rows: Vec<_> = pages.iter().map(per_group).collect();
        let groups: Vec<Vec<_>> =
            rows.iter().map(|page| page.iter().map(|rows| any(rows.clone())).collect()).collect();
        let kept: Rc<Vec<_>> = Rc::new(groups.iter().map(|page| any(page.clone())).collect());
        let searching = memo(move || !needle.with(String::is_empty));
        let pages_kept = kept.clone();
        let shown = memo(move || {
            let (at, keeps) = (current.get(), |i: &usize| pages_kept.get(*i).is_some_and(|kept| kept.get()));
            match !searching.get() || keeps(&at) {
                true => at,
                false => (0..pages_kept.len()).find(keeps).unwrap_or(at),
            }
        });
        let heads = pages.iter().map(|page| page.groups.iter().map(|_| NodeRef::new()).collect());
        Model {
            current,
            searching,
            rows: Rc::new(rows),
            groups: Rc::new(groups),
            pages: kept,
            shown,
            open: Rc::new(pages.iter().map(|_| signal(false)).collect()),
            picked: signal(None),
            scroll: signal(0.0),
            heads: Rc::new(heads.collect()),
            content: Rc::default(),
        }
    }

    /// What a click in the column does: the page is chosen, and its top shows; or a group of
    /// it is, and its header comes `lift` under the top once the layout knows where it is.
    fn go(&self, page: usize, group: Option<usize>, lift: f32) {
        self.current.set(page);
        self.picked.set(group.map(|group| (page, group)));
        let (Some(group), Some((handle, content))) = (group, self.content.get().cloned()) else {
            return self.scroll.set(0.0);
        };
        let (heads, scroll) = (self.heads.clone(), self.scroll);
        handle.with(move |tree| {
            tree.after_layout(move |tree| {
                if let Some(head) = heads[page][group].get() {
                    let down = tree.window_rect(head).y - tree.window_rect(content).y;
                    scroll.set((down - lift).max(0.0));
                }
            });
        });
    }

    /// The box the page scrolls says where it is, for [`Model::go`].
    fn scrolled(&self, content: Div) -> Div {
        let known = self.content.clone();
        content.on_mount(move |tree, id| _ = known.set((tree.handle(), id)))
    }
}

/// Displayed while `on` is true, and out of the layout otherwise.
fn shown(on: bool) -> Display {
    if on { Display::Flex } else { Display::None }
}

/// A row of the column does `f` on a click, and on Enter and Space with the focus.
fn acts(line: Div, f: impl Fn() + Clone + 'static) -> Div {
    let line = line.cursor(Cursor::Pointer).focusable().key_context(CONTROL);
    let line = line.focus_visible(|s| s.outline(token::BORDER_FOCUSED));
    let by_key = f.clone();
    line.on_action::<Confirm>(move |_| by_key()).on_click(move |_| f())
}

/// A page's row says whether its groups show under it, and a reader can show and hide them.
fn opens(line: Div, open: Signal<bool>) -> Div {
    line.expanded(move || open.get()).on_semantic(move |action| {
        match action {
            SemanticAction::Expand => open.set(true),
            SemanticAction::Collapse => open.set(false),
            _ => return false,
        }
        true
    })
}

/// The search field of the column, `width` wide.
fn search(query: Signal<String>, width: f32) -> Input {
    input(query).placeholder("Search settings…").icon(icons::SEARCH).width(width)
}

impl View for Settings {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let model =
            Model::new(&self.pages, self.current, self.query.unwrap_or_else(|| signal(String::new())));
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
