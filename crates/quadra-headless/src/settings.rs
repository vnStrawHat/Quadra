//! Settings (specs/17, "The components"): pages of groups of rows. A search keeps the rows
//! whose text holds it, whatever the case and the accents; a group and a page that keep no row
//! leave, and while the chosen page keeps none the first page that keeps one shows. A click in
//! the column chooses a page and shows its top, or a group and scrolls its header to the top.
//! The look gives the column, the pages, the groups and the rows.

use std::cell::OnceCell;
use std::rc::Rc;

use quadra_layout::Display;
use quadra_reactive::{Memo, Signal, memo, signal};
use quadra_text::fold_diacritics;
use quadra_tree::{Handle, NodeId, SemanticAction};
use quadra_view::{Div, NodeRef};

use crate::action::{CONTROL, Confirm};

/// What the column and the page share: what a search keeps, and what the component keeps.
#[derive(Clone)]
pub struct Model {
    pub current: Signal<usize>,
    pub searching: Memo<bool>,
    /// Whether a row is kept, by page, group and row; whether a group and a page keep one.
    pub rows: Rc<Vec<Vec<Vec<Memo<bool>>>>>,
    pub groups: Rc<Vec<Vec<Memo<bool>>>>,
    pub pages: Rc<Vec<Memo<bool>>>,
    /// The page at the right: the chosen one, or while a search leaves it nothing the first
    /// page that keeps a row.
    pub shown: Memo<usize>,
    /// The pages whose groups show under them in the column.
    pub open: Rc<Vec<Signal<bool>>>,
    /// The group chosen in the column: its page and its place there.
    pub picked: Signal<Option<(usize, usize)>>,
    pub scroll: Signal<f32>,
    /// What a chosen group is scrolled by: the header of each, and what the page scrolls.
    pub heads: Rc<Vec<Vec<NodeRef>>>,
    content: Rc<OnceCell<(Handle, NodeId)>>,
}

impl Model {
    /// The model of pages of groups of rows, each row known by the text a search looks in.
    pub fn new(pages: &[Vec<Vec<String>>], current: Signal<usize>, query: Signal<String>) -> Model {
        let plain = |text: &str| fold_diacritics(text).to_lowercase();
        let needle = memo(move || query.with(|query| plain(query)));
        let hit = |row: &String| {
            let hay = plain(row);
            memo(move || needle.with(|needle| needle.is_empty() || hay.contains(needle)))
        };
        let any = |all: Vec<Memo<bool>>| memo(move || all.iter().any(|kept| kept.get()));
        let per_group = |page: &Vec<Vec<String>>| -> Vec<Vec<Memo<bool>>> {
            page.iter().map(|group| group.iter().map(hit).collect()).collect()
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
        let heads = pages.iter().map(|page| page.iter().map(|_| NodeRef::new()).collect());
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
    pub fn go(&self, page: usize, group: Option<usize>, lift: f32) {
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
    pub fn scrolled(&self, content: Div) -> Div {
        let known = self.content.clone();
        content.on_mount(move |tree, id| _ = known.set((tree.handle(), id)))
    }
}

/// Displayed while `on` is true, and out of the layout otherwise.
pub fn shown(on: bool) -> Display {
    if on { Display::Flex } else { Display::None }
}

/// A row of the column does `f` on a click, and on Enter and Space with the focus.
pub fn acts(line: Div, f: impl Fn() + Clone + 'static) -> Div {
    let line = line.focusable().key_context(CONTROL);
    let by_key = f.clone();
    line.on_action::<Confirm>(move |_| by_key()).on_click(move |_| f())
}

/// A page's row says whether its groups show under it, and a reader can show and hide them.
pub fn opens(line: Div, open: Signal<bool>) -> Div {
    line.expanded(move || open.get()).on_semantic(move |action| {
        match action {
            SemanticAction::Expand => open.set(true),
            SemanticAction::Collapse => open.set(false),
            _ => return false,
        }
        true
    })
}
