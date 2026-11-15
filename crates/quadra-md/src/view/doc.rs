//! The document as a view (specs/13, "Markdown"): its blocks in a column, or in a list of
//! the blocks that show when it scrolls itself; where each block is; a line brought into
//! view.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use quadra_design::token;
use quadra_layout::Length;
use quadra_reactive::{Signal, effect, memo};
use quadra_tree::{NodeId, Tree};
use quadra_view::{Div, IntoProp, MountCx, Prop, View, column, dynamic, each, virtual_list};

use super::{BODY, Entry, OnLink, block, entries};

/// A block laid out: the source line it starts on, and its top and its height from the top
/// of the document.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub line: usize,
    pub top: f32,
    pub height: f32,
}

type Line = Box<dyn Fn() -> Option<usize>>;

pub struct Markdown {
    source: Prop<String>,
    on_link: Option<OnLink>,
    base: Option<PathBuf>,
    scroll: Option<Signal<f32>>,
    places: Option<Signal<Vec<Place>>>,
    reveal: Option<Line>,
}

/// A document, rendered. The source is a property: when it changes, the blocks whose source
/// did not change keep their nodes.
pub fn markdown<M>(source: impl IntoProp<String, M>) -> Markdown {
    let source = source.into_prop();
    Markdown { source, on_link: None, base: None, scroll: None, places: None, reveal: None }
}

impl Markdown {
    /// Runs `f` with the target of a link when it is clicked with Ctrl held.
    pub fn on_link(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.on_link = Some(Rc::new(f));
        self
    }

    /// The folder the document is in: where its images are looked for when their paths
    /// do not start at a root.
    pub fn base(mut self, folder: impl Into<PathBuf>) -> Self {
        self.base = Some(folder.into());
        self
    }

    /// The document scrolls itself, `scroll_y` far, and mounts only the blocks that show:
    /// for documents of tens of thousands of lines. Give it a height, or let it grow.
    pub fn scrolls(mut self, scroll_y: Signal<f32>) -> Self {
        self.scroll = Some(scroll_y);
        self
    }

    /// Writes where the blocks are after each layout that moved them: every block, or in a
    /// document that scrolls itself those that are mounted. For an outline, and for two
    /// panes that scroll together.
    pub fn places(mut self, out: Signal<Vec<Place>>) -> Self {
        self.places = Some(out);
        self
    }

    /// Brings the block a source line is in into view when `line` gives another one,
    /// scrolling as little as shows it.
    pub fn reveal(mut self, line: impl Fn() -> Option<usize> + 'static) -> Self {
        self.reveal = Some(Box::new(line));
        self
    }
}

/// The blocks mounted, by their nodes, with their keys.
#[derive(Clone, Default)]
struct Rows(Rc<RefCell<Vec<(NodeId, u64)>>>);

impl Rows {
    fn places(&self, tree: &Tree, page: NodeId, scrolled: f32, lines: &HashMap<u64, usize>) -> Vec<Place> {
        let mut rows = self.0.borrow_mut();
        rows.retain(|(id, _)| tree.is_alive(*id));
        let top = tree.window_rect(page).y - scrolled;
        let place = |&(id, key): &(NodeId, u64)| {
            let rect = tree.window_rect(id);
            Some(Place { line: *lines.get(&key)?, top: rect.y - top, height: rect.height })
        };
        let mut places: Vec<Place> = rows.iter().filter_map(place).collect();
        places.sort_by_key(|place| place.line);
        places
    }

    fn node(&self, key: u64) -> Option<NodeId> {
        self.0.borrow().iter().rev().find(|(_, own)| *own == key).map(|(id, _)| *id)
    }
}

impl View for Markdown {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (on_link, base, scroll) = (self.on_link, self.base, self.scroll);
        let read: Box<dyn Fn() -> String> = match self.source {
            Prop::Value(source) => Box::new(move || source.clone()),
            Prop::Reactive(read) => read,
        };
        let parsed = memo(move || Rc::new(entries(&read())));
        let blocks = memo(move || parsed.with(|parsed| parsed.0.clone()));
        let lines = memo(move || parsed.with(|parsed| parsed.1.clone()));
        let rows = Rows::default();
        let known = rows.clone();
        let row = move |entry: &dyn Fn() -> Entry| -> Div {
            let (key, kind) = (entry().key, entry().kind);
            let block = block(&kind, &on_link, base.as_deref());
            let known = known.clone();
            column().on_mount(move |_, id| known.0.borrow_mut().push((id, key))).child(block)
        };
        let page = column().width(Length::Fill).text_color(token::TEXT).text_size(BODY).line_height(1.5);
        // The block a line is in: the last that starts at or before it.
        let block_of = move |line: usize| {
            let lines = lines.get();
            blocks.with(|blocks| {
                blocks.iter().rposition(|entry| lines.get(&entry.key).is_some_and(|&at| at <= line))
            })
        };
        let id = match scroll {
            Some(scroll_y) => {
                let row = Rc::new(row);
                let list = virtual_list(
                    move || blocks.with(Vec::len),
                    48.0,
                    scroll_y,
                    move |index| {
                        let entry = memo(move || blocks.with(|blocks| blocks.get(index.get()).cloned()));
                        let row = row.clone();
                        dynamic(move || match entry.get() {
                            Some(entry) => {
                                let gap = entry.gap;
                                row(&move || entry.clone()).padding_top(gap)
                            }
                            None => column(),
                        })
                    },
                );
                let list = match self.reveal {
                    Some(line) => list.reveal(move || line().and_then(block_of)),
                    None => list,
                };
                list.container(page.grow(1.0)).mount(cx)
            }
            None => {
                let shown = move |entry: Signal<Entry>| {
                    row(&move || entry.get_untracked()).padding_top(move || entry.with(|entry| entry.gap))
                };
                let id = each(move || blocks.get(), |entry| entry.key, shown).container(page).mount(cx);
                if let Some(line) = self.reveal {
                    let (handle, rows) = (cx.handle.clone(), rows.clone());
                    effect(move || {
                        let key = line()
                            .and_then(block_of)
                            .and_then(|at| blocks.with(|b| b.get(at).map(|e| e.key)));
                        let Some(node) = key.and_then(|key| rows.node(key)) else { return };
                        handle.with(move |tree| tree.after_layout(move |tree| tree.scroll_into_view(node)));
                    });
                }
                id
            }
        };
        super::select::selectable(cx, id);
        if let Some(out) = self.places {
            // After a layout that moved the blocks: the page changed size, or scrolled.
            let (size, handle) = (cx.tree.size_signal(id), cx.handle.clone());
            effect(move || {
                size.get();
                let scrolled = scroll.map_or(0.0, |scroll_y| scroll_y.get());
                let (rows, lines) = (rows.clone(), lines.get());
                handle.with(move |tree| {
                    tree.after_layout(move |tree| {
                        let places = rows.places(tree, id, scrolled, &lines);
                        if out.with_untracked(|was| *was != places) {
                            out.set(places);
                        }
                    })
                });
            });
        }
        id
    }
}
