//! The editor and the pointer: a press puts the cursor (Shift extends it, Ctrl adds one), a
//! drag selects to where it goes, Alt and a drag a column; two clicks a word, three the line.
//! A link under the pointer is underlined while Ctrl is held, and Ctrl and a press open it;
//! once the pointer has rested on one, a hint under it says where it goes and how.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

use quadra_async::{Timer, timeout};
use quadra_headless::hint::TOOLTIP_REST;
use quadra_headless::popup::hang_with;
use quadra_platform::{Mods, MouseButton};
use quadra_prim::Point;
use quadra_reactive::{Scope, Signal, effect, signal};
use quadra_tree::{Anchor, Handle, NodeId, PointerEvent, PointerKind};
use quadra_view::MountCx;

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

/// The link under the pointer: where its hint shows, in the window, and its URL.
type Over = Signal<Option<(Point, Rc<str>)>>;

pub(super) fn pointer(cx: &mut MountCx, id: NodeId, epoch: Instant, on_link: Option<OnLink>) {
    // Sideways: the wheel turned so, or with Shift.
    let writer = cx.handle.clone();
    cx.tree.on_pointer(id, PointerKind::Wheel, move |e| {
        let dx = if e.mods.contains(Mods::SHIFT) { -e.delta.y } else { -e.delta.x };
        if dx != 0.0 {
            e.stop();
            writer.with(move |tree| {
                let moved = |w: &mut EditorWidget| (w.scroll_by(dx), w.tell_sideways()).0;
                if tree.widget_mut::<EditorWidget>(id).is_some_and(moved) {
                    tree.mark_paint(id);
                }
            });
        }
    });
    let pressed: Pressed = Rc::new(Cell::new(None));
    // Whether the button is down, as the events come; `pressed` is filled as they are
    // handled, in the update pass.
    let held = Rc::new(Cell::new(false));
    let over = on_link.is_some().then(|| hint(cx, id));
    let handler = move |writer: Handle, pressed: Pressed, held: Rc<Cell<bool>>| {
        let on_link = on_link.clone();
        move |e: &mut PointerEvent| {
            if e.button != MouseButton::Left && e.kind != PointerKind::Move {
                return;
            }
            let (at, kind, clicks, mods) = (e.local, e.kind, e.clicks, e.mods);
            let window = Point::new(e.at.x, e.at.y + 16.0);
            if let (Some(over), PointerKind::Down) = (over, kind) {
                over.set(None);
            }
            if kind == PointerKind::Move && !held.get() {
                // The bar along the bottom learns here how far the lines painted since go.
                writer.with(move |tree| {
                    if let Some(over) = over {
                        hover(tree, id, at, mods, (window, over));
                    }
                    tree.widget_mut::<EditorWidget>(id).into_iter().for_each(|w| w.tell_sideways());
                });
                return;
            }
            if kind == PointerKind::Down
                && mods.contains(Mods::CTRL)
                && let Some(on_link) = on_link.clone()
            {
                // On a link, the press is the link's: no cursor moves or is added.
                let time = writer.now();
                writer.with(move |tree| {
                    if let Some(over) = over {
                        hover(tree, id, at, mods, (window, over));
                    }
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
                PointerKind::Down => {
                    held.set(true);
                    e.capture_pointer();
                }
                // After the press, which waits for the update pass too: a press and a release
                // in one frame leave nothing held.
                PointerKind::Up => {
                    held.set(false);
                    let pressed = pressed.clone();
                    return writer.with(move |_| pressed.set(None));
                }
                _ => {}
            }
            let (pressed, time) = (pressed.clone(), writer.now());
            writer.with(move |tree| {
                edit(tree, id, (epoch, time), |w, _| press(w, &pressed, at, (kind, clicks), mods));
            });
        }
    };
    for kind in [PointerKind::Down, PointerKind::Move, PointerKind::Up, PointerKind::Click] {
        cx.tree.on_pointer(id, kind, handler(cx.handle.clone(), pressed.clone(), held.clone()));
    }
}

/// The hint over a link: its URL and how to open it, under the pointer once it has rested
/// on one link; another link, or none, starts again. What `over` is set to shows.
fn hint(cx: &mut MountCx, id: NodeId) -> Over {
    let (over, shown): (Over, _) = (signal(None), signal(false));
    let (rest, owner): (Rc<RefCell<Option<Timer>>>, _) = (Rc::default(), Scope::current());
    effect(move || {
        let on = over.with(Option::is_some);
        shown.set(false);
        if let Some(timer) = rest.borrow_mut().take() {
            timer.cancel();
        }
        *rest.borrow_mut() = on.then(|| owner.run(|| timeout(TOOLTIP_REST, move || shown.set(true))));
    });
    let under = move || over.with(|link| link.as_ref().map_or(Point::ZERO, |(at, _)| *at));
    let says = move || over.with(|link| link.as_ref().map(|(_, url)| format!("{url} (Ctrl-click to open)")));
    let tip = move || quadra_design::tip(&says().unwrap_or_default());
    hang_with(cx, id, shown, move || Anchor::At(under()), tip);
    over
}

/// Finds the link under the pointer, on its line alone, and underlines it while Ctrl is
/// held, with the pointer of a link; tells `over` of another link, with where its hint goes.
fn hover(tree: &mut quadra_tree::Tree, id: NodeId, at: Point, mods: Mods, (window, over): (Point, Over)) {
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
    if w.link != link {
        over.set(link.as_ref().map(|(_, url)| (window, Rc::from(url.as_str()))));
    }
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
