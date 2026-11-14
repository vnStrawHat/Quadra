//! What the editor's view keeps up with as it goes: the caret's blink while it has the focus,
//! and the line at the top of the view, for what follows the editor (a preview).

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use quadra_async::{Timers, interval};
use quadra_reactive::{Scope, Signal, use_context};
use quadra_tree::NodeId;
use quadra_ui::MountCx;

use super::EditorWidget;

const CARET_BLINK: Duration = Duration::from_millis(530);

/// While it has the focus the caret blinks, on a timer that lives as long as the focus.
pub(super) fn blink(cx: &mut MountCx, id: NodeId) {
    let (owner, blinks) = (Scope::current(), use_context::<Rc<Timers>>().is_some());
    let session: Cell<Option<Scope>> = Cell::new(None);
    let writer = cx.handle.clone();
    cx.tree.on_focus(id, move |gained| {
        if let Some(ended) = session.take() {
            ended.dispose();
        }
        if gained && blinks {
            let scope = owner.run(Scope::child);
            let writer = writer.clone();
            scope.run(|| {
                interval(CARET_BLINK, move || {
                    writer.with(move |tree| {
                        if let Some(w) = tree.widget_mut::<EditorWidget>(id) {
                            w.caret_on = !w.caret_on;
                            tree.mark_paint(id);
                        }
                    })
                })
            });
            session.set(Some(scope));
        }
        writer.with(move |tree| {
            if let Some(w) = tree.widget_mut::<EditorWidget>(id) {
                (w.focused, w.caret_on) = (gained, true);
                tree.mark_paint(id);
            }
        });
    });
}

/// Keeps `top` the line at the top of the view as `scroll_y` moves, and scrolls the view to
/// put a line at its top when `top` is set to it (as far as the view goes).
pub(super) fn follow_top(
    cx: &mut MountCx,
    widget: Rc<Cell<Option<NodeId>>>,
    scroll_y: Signal<f32>,
    top: Signal<usize>,
) {
    // The line at a height, and the height of a line, by the editor's rows.
    let line_at = |w: &mut EditorWidget, y: f32| {
        w.sync_rows();
        let last = w.row_count().saturating_sub(1);
        w.row_line(((y / w.line_px).max(0.0) as usize).min(last))
    };
    let (writer, slot) = (cx.handle.clone(), widget.clone());
    cx.handle.bind(move || {
        let y = scroll_y.get();
        let slot = slot.clone();
        writer.with(move |tree| {
            let w = slot.get().and_then(|id| tree.widget_mut::<EditorWidget>(id));
            if let Some(line) = w.map(|w| line_at(w, y)).filter(|&line| line != top.get_untracked()) {
                top.set(line);
            }
        });
    });
    let writer = cx.handle.clone();
    cx.handle.bind(move || {
        let line = top.get();
        let widget = widget.clone();
        writer.with(move |tree| {
            let Some(id) = widget.get() else { return };
            let view = tree.parent(id).map_or(0.0, |port| tree.rect(port).height);
            let Some(w) = tree.widget_mut::<EditorWidget>(id) else { return };
            if line_at(w, scroll_y.get_untracked()) != line {
                // As far as the view goes: the line it then shows at its top is told.
                let line = line.min(w.doc.buffer.line_count().saturating_sub(1));
                let end = (w.row_count() as f32 * w.line_px - view).max(0.0);
                scroll_y.set((w.line_rows(line).start as f32 * w.line_px).min(end));
            }
        });
    });
}
