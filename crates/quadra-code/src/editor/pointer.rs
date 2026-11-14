//! The editor and the pointer: a press puts the cursor (Shift extends it, Ctrl adds one), a
//! drag selects to where it goes, Alt and a drag a column; two clicks a word, three the line.
//! A link under the pointer is underlined while Ctrl is held, and Ctrl and a press open it.

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

/// What opens a link: the app's.
pub(super) type OnLink = Rc<dyn Fn(&str)>;

pub(super) fn pointer(cx: &mut MountCx, id: NodeId, epoch: Instant, on_link: Option<OnLink>) {
    let pressed: Pressed = Rc::new(Cell::new(None));
    let handler = move |writer: Handle, pressed: Pressed| {
        let on_link = on_link.clone();
        move |e: &mut PointerEvent| {
            if e.button != MouseButton::Left && e.kind != PointerKind::Move {
                return;
            }
            let (at, kind, clicks, mods) = (e.local, e.kind, e.clicks, e.mods);
            if kind == PointerKind::Move && pressed.get().is_none() {
                if on_link.is_some() {
                    writer.with(move |tree| hover(tree, id, at, mods));
                }
                return;
            }
            if kind == PointerKind::Down
                && mods.contains(Mods::CTRL)
                && let Some(on_link) = on_link.clone()
            {
                // On a link, the press is the link's: no cursor moves or is added.
                let time = writer.now();
                writer.with(move |tree| {
                    hover(tree, id, at, mods);
                    let w = tree.widget_mut::<EditorWidget>(id);
                    match w.and_then(|w| w.link.clone().filter(|_| w.link_hot)) {
                        Some((_, url)) => on_link(&url),
                        None => {
                            let pressed = Cell::new(None);
                            edit(tree, id, (epoch, time), |w, _| {
                                press(w, &pressed, at, (kind, clicks), mods)
                            });
                        }
                    }
                });
                return;
            }
            match kind {
                PointerKind::Down => e.capture_pointer(),
                PointerKind::Up => return pressed.set(None),
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

/// Finds the link under the pointer, on its line alone, and underlines it while Ctrl is
/// held, with the pointer of a link.
fn hover(tree: &mut quadra_tree::Tree, id: NodeId, at: Point, mods: Mods) {
    let Some(w) = tree.widget_mut::<EditorWidget>(id) else { return };
    let offset = w.offset_at(at);
    let line = w.doc.buffer.line_of(offset);
    let start = w.doc.buffer.line_start(line);
    let urls = quadra_text::find_urls(&w.doc.buffer.line(line));
    let link = urls
        .into_iter()
        .map(|u| (start + u.range.start..start + u.range.end, u.url))
        .find(|(range, _)| range.contains(&offset));
    let hot = mods.contains(Mods::CTRL) && link.is_some();
    if w.link != link || w.link_hot != hot {
        (w.link, w.link_hot) = (link, hot);
        tree.mark_paint(id);
        let cursor = if hot { quadra_platform::Cursor::Pointer } else { quadra_platform::Cursor::Text };
        tree.set_cursor(id, Some(cursor));
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
