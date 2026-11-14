//! The editor and the pointer: a press puts the cursor (Shift extends it, Ctrl adds one), a
//! drag selects to where it goes, Alt and a drag a column; two clicks a word, three the line.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use quadra_platform::{Mods, MouseButton};
use quadra_prim::Point;
use quadra_tree::{Handle, NodeId, PointerEvent, PointerKind};
use quadra_ui::MountCx;

use super::{EditorWidget, edit};
use crate::doc::Cursor;

/// Where the press was, while the button is down.
#[derive(Clone, Copy)]
struct Press {
    anchor: usize,
    line: usize,
    col: usize,
    /// It selects a column.
    column: bool,
}

type Pressed = Rc<Cell<Option<Press>>>;

pub(super) fn pointer(cx: &mut MountCx, id: NodeId, epoch: Instant) {
    let pressed: Pressed = Rc::new(Cell::new(None));
    let handler = move |writer: Handle, pressed: Pressed| {
        move |e: &mut PointerEvent| {
            if e.button != MouseButton::Left && e.kind != PointerKind::Move {
                return;
            }
            let (at, kind, clicks, mods) = (e.local, e.kind, e.clicks, e.mods);
            match kind {
                PointerKind::Down => e.capture_pointer(),
                PointerKind::Up => return pressed.set(None),
                PointerKind::Move if pressed.get().is_none() => return,
                _ => {}
            }
            let (pressed, time) = (pressed.clone(), writer.now());
            writer.with(move |tree| {
                edit(tree, id, (epoch, time), |w, _| press(w, &pressed, at, (kind, clicks), mods));
            });
        }
    };
    for kind in [PointerKind::Down, PointerKind::Move, PointerKind::Up, PointerKind::Click] {
        cx.tree.on_pointer(id, kind, handler(cx.handle.clone(), pressed.clone()));
    }
}

fn press(
    w: &mut EditorWidget,
    pressed: &Cell<Option<Press>>,
    at: Point,
    event: (PointerKind, u8),
    mods: Mods,
) {
    let offset = w.offset_at(at);
    let (line, col) = (w.doc.buffer.line_of(offset), w.col_at(at.x));
    match event {
        (PointerKind::Click, 2) => {
            let word = w.doc.word_at(offset);
            w.doc.set_cursor(word.start, word.end, false);
        }
        (PointerKind::Click, 3..) => {
            w.doc.set_cursor(offset, offset, false);
            w.doc.select_line();
        }
        (PointerKind::Click, _) => {}
        (PointerKind::Down, _) => {
            if mods.contains(Mods::SHIFT) {
                let anchor = w.doc.primary().anchor;
                w.doc.set_cursor(anchor, offset, false);
            } else {
                w.doc.set_cursor(offset, offset, mods.contains(Mods::CTRL));
            }
            let anchor = w.doc.primary().anchor;
            pressed.set(Some(Press { anchor, line, col, column: mods.contains(Mods::ALT) }));
        }
        _ => match pressed.get() {
            // A column: a cursor a line, from the column of the press to that of the pointer.
            Some(Press { line: from, col: from_col, column: true, .. }) => {
                w.doc.cursors.clear();
                for n in from.min(line)..=from.max(line) {
                    let (anchor, head) = (w.doc.offset_at_col(n, from_col), w.doc.offset_at_col(n, col));
                    w.doc.cursors.push(Cursor { anchor, head, goal: None });
                }
                w.doc.normalize();
            }
            Some(Press { anchor, .. }) => {
                let last = w.doc.cursors.len() - 1;
                (w.doc.cursors[last].anchor, w.doc.cursors[last].head) = (anchor, offset);
                w.doc.cursors[last].goal = None;
                w.doc.normalize();
            }
            None => {}
        },
    }
}
