//! One selection over the blocks of a document (specs/13, "Markdown"): a press in a text and
//! a drag to another select from the one to the other, the texts between them whole; Ctrl-C
//! copies it, Ctrl-A selects all of it. Its texts are the rich texts under the page, in the
//! order of the tree, which is that of the document.

use std::cell::Cell;
use std::rc::Rc;

use quadra_platform::{Clipboard, Cursor, Mods, MouseButton};
use quadra_prim::Point;
use quadra_reactive::use_context;
use quadra_tree::{NodeId, PointerEvent, PointerKind, RichText, TEXT_SELECT_CONTEXT, TextCommand, Tree};
use quadra_ui::MountCx;

/// Where a selection ends: a text, and an offset in it.
type End = (NodeId, usize);

/// The rich texts under a node, in the order of the tree.
fn texts(tree: &mut Tree, page: NodeId) -> Vec<NodeId> {
    let (mut out, mut stack) = (Vec::new(), vec![page]);
    while let Some(at) = stack.pop() {
        if tree.widget_mut::<RichText>(at).is_some() {
            out.push(at);
        }
        stack.extend(tree.children(at).iter().rev().copied());
    }
    out
}

/// The text under a point of the window, or the nearest above or below it, and the offset
/// in it nearest the point.
fn end_at(tree: &mut Tree, page: NodeId, at: Point) -> Option<End> {
    let texts = texts(tree, page);
    let distance = |tree: &Tree, id: NodeId| {
        let rect = tree.window_rect(id);
        let dy = if at.y < rect.y { rect.y - at.y } else { (at.y - rect.bottom()).max(0.0) };
        let dx = if at.x < rect.x { rect.x - at.x } else { (at.x - rect.right()).max(0.0) };
        dy * 1000.0 + dx
    };
    let id = texts.into_iter().min_by(|a, b| distance(tree, *a).total_cmp(&distance(tree, *b)))?;
    let rect = tree.window_rect(id);
    let local = Point::new(at.x - rect.x, (at.y - rect.y).clamp(0.0, rect.height.max(1.0) - 1.0));
    Some((id, tree.widget_mut::<RichText>(id)?.index_at(local)))
}

/// Selects from `from` to `to` (none: nothing), and paints again the texts that changed.
fn apply(tree: &mut Tree, page: NodeId, ends: Option<(End, End)>) {
    let texts = texts(tree, page);
    let place = |(id, at): End| texts.iter().position(|&t| t == id).map(|k| (k, at));
    let ends = ends.and_then(|(a, b)| Some((place(a)?, place(b)?)));
    let (start, end) = match ends {
        Some((a, b)) if a <= b => (a, b),
        Some((a, b)) => (b, a),
        None => ((usize::MAX, 0), (usize::MAX, 0)),
    };
    for (k, &id) in texts.iter().enumerate() {
        let Some(text) = tree.widget_mut::<RichText>(id) else { continue };
        let all = text.len();
        let range = match k {
            _ if start.0 == usize::MAX || k < start.0 || k > end.0 => 0..0,
            _ if k == start.0 && k == end.0 => start.1..end.1,
            _ if k == start.0 => start.1..all,
            _ if k == end.0 => 0..end.1,
            _ => 0..all,
        };
        if text.select(range) {
            tree.mark_paint(id);
        }
    }
}

/// What is selected, a text a line.
fn selected(tree: &mut Tree, page: NodeId) -> String {
    let texts = texts(tree, page);
    let parts = texts.iter().filter_map(|&id| tree.widget_mut::<RichText>(id).map(|text| text.selected()));
    parts.filter(|part| !part.is_empty()).collect::<Vec<_>>().join("\n")
}

/// Makes the document mounted at `page` selectable.
pub(super) fn selectable(cx: &mut MountCx, page: NodeId) {
    cx.tree.set_focusable(page, true);
    cx.tree.set_cursor(page, Some(Cursor::Text));
    cx.tree.set_key_context(page, TEXT_SELECT_CONTEXT);
    let anchor: Rc<Cell<Option<End>>> = Rc::default();
    let dragging = Rc::new(Cell::new(false));
    for kind in [PointerKind::Down, PointerKind::Move, PointerKind::Up] {
        let (handle, anchor, dragging) = (cx.handle.clone(), anchor.clone(), dragging.clone());
        cx.tree.on_pointer(page, kind, move |e: &mut PointerEvent| {
            // Ctrl and a click is for links.
            let left = e.button == MouseButton::Left && !e.mods.contains(Mods::CTRL);
            let at = e.at;
            match e.kind {
                PointerKind::Down if left => {
                    dragging.set(true);
                    e.capture_pointer();
                    let anchor = anchor.clone();
                    handle.with(move |tree| {
                        let end = end_at(tree, page, at);
                        anchor.set(end);
                        apply(tree, page, end.map(|end| (end, end)));
                    });
                }
                PointerKind::Move if dragging.get() => {
                    let anchor = anchor.clone();
                    handle.with(move |tree| {
                        let ends = anchor.get().zip(end_at(tree, page, at));
                        apply(tree, page, ends);
                    });
                }
                PointerKind::Up => dragging.set(false),
                _ => {}
            }
        });
    }
    let (clipboard, handle) = (use_context::<Rc<dyn Clipboard>>(), cx.handle.clone());
    cx.tree.on_action::<TextCommand>(page, move |&command| {
        let clipboard = clipboard.clone();
        handle.with(move |tree| match (command, clipboard) {
            (TextCommand::SelectAll, _) => {
                let texts = texts(tree, page);
                let last = texts.last().and_then(|&id| Some((id, tree.widget_mut::<RichText>(id)?.len())));
                apply(tree, page, texts.first().map(|&id| (id, 0)).zip(last));
            }
            (TextCommand::Copy, Some(clipboard)) => {
                let said = selected(tree, page);
                if !said.is_empty() {
                    clipboard.write_text(&said);
                }
            }
            _ => {}
        });
    });
    let handle = cx.handle.clone();
    cx.tree.on_focus(page, move |gained| {
        if !gained {
            handle.with(move |tree| apply(tree, page, None));
        }
    });
}
