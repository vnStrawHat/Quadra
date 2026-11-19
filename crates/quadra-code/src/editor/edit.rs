//! What edits the editor's widget, from its own events and from outside it: the one place a
//! change is followed by what it implies (colours, marks, layout, the caret in view).

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use quadra_tree::{Handle, NodeId, Tree};

use super::EditorWidget;

/// Where the primary cursor is, counted from 1, and how many characters it selects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Caret {
    pub line: usize,
    pub column: usize,
    pub selected: usize,
}

impl Caret {
    fn of(w: &EditorWidget) -> Caret {
        let (cursor, buffer) = (w.doc.primary(), &w.doc.buffer);
        let line = buffer.line_of(cursor.head);
        let column = buffer.slice(buffer.line_start(line)..cursor.head).chars().count() + 1;
        let range = cursor.anchor.min(cursor.head)..cursor.anchor.max(cursor.head);
        Caret { line: line + 1, column, selected: buffer.slice(range).chars().count() }
    }
}

/// Reaches an editor from outside it while it is mounted: a toolbar, a save. Copies are the
/// same one.
#[derive(Clone, Default)]
pub struct EditorHandle(Rc<RefCell<Option<Reach>>>);

#[derive(Clone)]
struct Reach {
    handle: Handle,
    id: NodeId,
    epoch: Instant,
}

impl EditorHandle {
    /// The editor is mounted: what is asked of it is done.
    pub fn is_mounted(&self) -> bool {
        self.0.borrow().is_some()
    }

    /// Runs `f` on the editor's widget in the tree's next pass; nothing while it is not
    /// mounted.
    pub fn read(&self, f: impl FnOnce(&mut EditorWidget) + 'static) {
        let Some(reach) = self.0.borrow().clone() else { return };
        reach.handle.with(move |tree| {
            if let Some(w) = tree.widget_mut::<EditorWidget>(reach.id) {
                f(w);
            }
        });
    }

    /// Changes the editor in the tree's next pass as a key would: then it is coloured,
    /// marked, laid out and painted again, with the caret in view.
    pub fn edit(&self, f: impl FnOnce(&mut EditorWidget) + 'static) {
        let Some(reach) = self.0.borrow().clone() else { return };
        let at = reach.handle.now();
        reach.handle.with(move |tree| edit(tree, reach.id, (reach.epoch, at), |w, _| f(w)));
    }

    /// Gives the editor the focus, in the tree's next pass: after a toolbar's button took it.
    pub fn focus(&self) {
        let Some(reach) = self.0.borrow().clone() else { return };
        reach.handle.with(move |tree| tree.focus(reach.id));
    }

    /// The editor `id` is mounted; it is forgotten when its view goes.
    pub(super) fn mounted(&self, handle: Handle, id: NodeId, epoch: Instant) {
        *self.0.borrow_mut() = Some(Reach { handle, id, epoch });
        let slot = self.0.clone();
        quadra_reactive::on_cleanup(move || {
            let mut reach = slot.borrow_mut();
            if reach.as_ref().is_some_and(|it| it.id == id) {
                *reach = None;
            }
        });
    }
}

/// Runs `f` on the editor's widget, as at `at` on the window's clock (when the event came),
/// then has it painted again (laid out again when its count of lines changed) with its
/// caret in view.
pub(super) fn edit(
    tree: &mut Tree,
    id: NodeId,
    (epoch, at): (Instant, Instant),
    f: impl FnOnce(&mut EditorWidget, usize),
) {
    let now = at.saturating_duration_since(epoch).as_millis() as u64;
    // A page is the lines of the view the editor scrolls in.
    let view = tree.parent(id).map_or(0.0, |port| tree.rect(port).height);
    let Some(w) = tree.widget_mut::<EditorWidget>(id) else { return };
    w.sync_rows();
    let (lines, version, rows) = (w.doc.buffer.line_count(), w.doc.version, w.row_count());
    let held = (w.doc.primary().anchor, w.doc.primary().head);
    w.doc.now = now;
    f(w, ((view / w.line_px) as usize).max(1));
    w.caret_on = true; // a caret in use does not blink away
    w.follow_caret();
    w.sync_rows();
    let changed = w.doc.version != version;
    let moved = (w.doc.primary().anchor, w.doc.primary().head) != held;
    if let Some(caret) = w.caret_line.filter(|_| moved || changed) {
        caret.try_set(w.doc.buffer.line_of(w.doc.primary().head));
    }
    if let Some(place) = w.caret.filter(|_| moved || changed) {
        place.try_set(Caret::of(w));
    }
    let relaid = w.doc.buffer.line_count() != lines || w.row_count() != rows;
    if changed && w.find.is_some() {
        w.refind_changed(); // the matches of the text as it is now
    }
    if changed {
        w.remark();
        if let Some(edited) = w.edited {
            edited.try_set(w.doc.version);
        }
    }
    let caret = w.caret_rect();
    if relaid {
        tree.mark_layout(id);
    }
    tree.mark_paint(id);
    if changed || relaid || moved {
        tree.update_semantics(id, |_| {}); // the text, the caret and the selection, for a reader
    }
    if relaid {
        tree.after_layout(move |tree| tree.scroll_rect_into_view(id, caret));
    } else {
        tree.scroll_rect_into_view(id, caret);
    }
}

/// How many rows a motion goes, down; none for one that is not up or down.
pub(super) fn rows_by(motion: crate::doc::Move, page: usize) -> Option<isize> {
    use crate::doc::Move;
    match motion {
        Move::Up => Some(-1),
        Move::Down => Some(1),
        Move::PageUp => Some(-(page as isize)),
        Move::PageDown => Some(page as isize),
        _ => None,
    }
}
