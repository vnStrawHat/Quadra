//! Find in a document (specs/13, "Markdown"): where a query matches in each of its texts,
//! marked apart from the selection, how many, and the current one brought into view.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_code::Query;
use quadra_design::token;
use quadra_reactive::{Signal, effect};
use quadra_tree::{NodeId, RichText, Tree};
use quadra_view::MountCx;

/// What is looked for, which match is the current one (from 0), and how many the document
/// found.
#[derive(Clone, Copy)]
pub struct Find {
    pub query: Signal<Option<Query>>,
    pub current: Signal<usize>,
    pub count: Signal<usize>,
}

/// Marks the matches after each layout that moved the blocks of `page` (`moved` reads what
/// says so), and when the query or the current one change.
pub(super) fn follow(cx: &mut MountCx, page: NodeId, f: Find, moved: impl Fn() + 'static) {
    let (size, handle) = (cx.tree.size_signal(page), cx.handle.clone());
    // What was last brought into view: it is again only when that changes.
    let shown: Rc<RefCell<Option<(Query, usize)>>> = Rc::default();
    effect(move || {
        size.get();
        moved();
        let (query, current, shown) = (f.query.get(), f.current.get(), shown.clone());
        handle.with(move |tree| {
            tree.after_layout(move |tree| {
                if tree.is_alive(page) {
                    mark(tree, page, query, current, f.count, &shown);
                }
            })
        });
    });
}

fn mark(
    tree: &mut Tree,
    page: NodeId,
    query: Option<Query>,
    current: usize,
    count: Signal<usize>,
    shown: &RefCell<Option<(Query, usize)>>,
) {
    let theme = tree.theme().clone();
    let (fill, chosen) = (theme.color(token::EDITOR_FIND), theme.color(token::EDITOR_FIND_CURRENT));
    // ponytail: every text is searched again on each change, a document's few thousand lines
    // in a millisecond or two; keep the matches by block if that shows.
    let (mut total, mut target) = (0, None);
    for id in super::select::texts(tree, page) {
        let Some(text) = tree.widget_mut::<RichText>(id) else { continue };
        let found = query.as_ref().and_then(|q| q.find_text(text.text()).ok()).unwrap_or_default();
        let here = (current >= total && current < total + found.len()).then(|| current - total);
        if let Some(k) = here {
            target = Some((id, found[k].clone()));
        }
        total += found.len();
        if text.mark(found, here, fill, chosen) {
            tree.mark_paint(id);
        }
    }
    if count.get_untracked() != total {
        count.set(total);
    }
    let now = query.map(|q| (q, current));
    if *shown.borrow() == now {
        return;
    }
    *shown.borrow_mut() = now;
    if let Some((id, range)) = target
        && let Some(rect) = tree.widget_mut::<RichText>(id).and_then(|text| text.rect_of(range))
    {
        tree.scroll_rect_into_view(id, rect);
    }
}
