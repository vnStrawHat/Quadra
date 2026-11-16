//! The terminal and the pointer (specs/18, "Input"): with mouse reporting on and Shift not
//! held, presses, drags and the wheel go to the program; otherwise a press and a drag select
//! (two clicks a word, three the line, Alt a block; copied on release when the settings say),
//! the middle button pastes when they say, a right press is left to the app's menu, and the
//! wheel scrolls the history, or sends arrows on the alternate screen.

use std::cell::Cell;
use std::rc::Rc;

use oneterm_vt::input::{
    KeyMods, KeySpec, MouseModifiers, NamedKey, TerminalMouseButton, encode_mouse_move, encode_mouse_press,
    encode_mouse_release, encode_wheel_event,
};
use oneterm_vt::{Pos, SelectionKind, Side};
use quadra_platform::{Clipboard, Mods, MouseButton};
use quadra_prim::Point;
use quadra_reactive::{Signal, use_context};
use quadra_tree::{NodeId, PointerEvent, PointerKind, Tree};
use quadra_view::MountCx;

use crate::actions::TermSettings;
use crate::link::TermLink;
use crate::session::Session;
use crate::view::send;
use crate::widget::TerminalWidget;

/// Where a point of the node is in the grid: a row and a column, as fractions.
pub(crate) fn grid_at(tree: &mut Tree, id: NodeId, at: Point) -> Option<(f32, f32)> {
    let w = tree.widget_mut::<TerminalWidget>(id)?;
    Some(((at.y - w.pad.top) / w.line, (at.x - w.gutter_width() - w.pad.left) / w.cell))
}

fn button(b: MouseButton) -> Option<TerminalMouseButton> {
    match b {
        MouseButton::Left => Some(TerminalMouseButton::Left),
        MouseButton::Middle => Some(TerminalMouseButton::Middle),
        MouseButton::Right => Some(TerminalMouseButton::Right),
        _ => None,
    }
}

pub(crate) fn pointer(
    cx: &mut MountCx,
    id: NodeId,
    session: &Session,
    settings: Option<Signal<TermSettings>>,
    link: Option<Rc<dyn Fn(TermLink)>>,
) {
    let settings = move || settings.map(|it| it.get_untracked()).unwrap_or_default();
    let clipboard = use_context::<Rc<dyn Clipboard>>();
    // The button held while the program reports, whether a selection is being dragged, and
    // whether the press followed a link (its moves and release are then nothing).
    let reported: Rc<Cell<Option<TerminalMouseButton>>> = Rc::new(Cell::new(None));
    let (selecting, linked) = (Rc::new(Cell::new(false)), Rc::new(Cell::new(false)));
    let hovering = link.is_some().then(|| crate::link::follow_ctrl(cx, id));
    for kind in [PointerKind::Down, PointerKind::Move, PointerKind::Up] {
        let (writer, s, clipboard) = (cx.handle.clone(), session.clone(), clipboard.clone());
        let (reported, selecting, linked) = (reported.clone(), selecting.clone(), linked.clone());
        let (link, hovering) = (link.clone(), hovering.clone());
        cx.tree.on_pointer(id, kind, move |e: &mut PointerEvent| {
            let (at, mods, clicks, pressed) = (e.local, e.mods, e.clicks, button(e.button));
            let reports = s.engine().term.mouse_reporting().is_some() && !mods.contains(Mods::SHIFT);
            match (kind, &hovering) {
                (PointerKind::Move, Some(hovering)) => {
                    hovering.moved(&writer, id, Some(at), mods.contains(Mods::CTRL))
                }
                (PointerKind::Down, _) => _ = s.0.pressed.take(),
                _ => {}
            }
            match kind {
                PointerKind::Down if pressed.is_none() => return,
                // Not the program's: the right button is the app's menu's (told the link under
                // it), the middle pastes.
                PointerKind::Down if !reports && pressed == Some(TerminalMouseButton::Right) => {
                    let s = s.clone();
                    return writer
                        .with(move |tree| *s.0.pressed.borrow_mut() = crate::link::under(tree, id, at));
                }
                PointerKind::Down if !reports && pressed == Some(TerminalMouseButton::Middle) => {
                    if let (true, Some(clipboard)) = (settings().middle_paste, &clipboard) {
                        e.stop();
                        s.paste(&clipboard.read_text().unwrap_or_default());
                    }
                    return;
                }
                PointerKind::Down => e.capture_pointer(),
                PointerKind::Move if reported.get().is_none() && !selecting.get() && !reports => return,
                _ => {}
            }
            e.stop();
            // Who the event is for is settled as it comes: the handling waits for the update.
            let (to_program, selects, held_button) =
                (reports && !selecting.get(), selecting.get(), reported.get());
            match kind {
                PointerKind::Down if to_program => reported.set(pressed),
                PointerKind::Down => selecting.set(pressed == Some(TerminalMouseButton::Left)),
                PointerKind::Up => (reported.set(None), selecting.set(false)).1,
                _ => {}
            }
            let copies = (kind == PointerKind::Up && selects && settings().copy_on_select)
                .then(|| clipboard.clone())
                .flatten();
            let follows = kind == PointerKind::Down
                && pressed == Some(TerminalMouseButton::Left)
                && mods.contains(Mods::CTRL)
                && link.is_some();
            let (s, reported, selecting, linked, link) =
                (s.clone(), reported.clone(), selecting.clone(), linked.clone(), link.clone());
            writer.with(move |tree| {
                // Ctrl+click on a link follows it, and the press is nothing else.
                if follows && let (Some(f), Some(found)) = (&link, crate::link::under(tree, id, at)) {
                    reported.set(None);
                    selecting.set(false);
                    linked.set(true);
                    return f(found);
                }
                if kind == PointerKind::Down {
                    linked.set(false);
                } else if linked.get() {
                    return linked.set(kind != PointerKind::Up);
                }
                let Some((row, col)) = grid_at(tree, id, at) else { return };
                let cell = (row.max(0.0) as usize, col.max(0.0) as usize);
                let modes = s.engine().term.mode_snapshot();
                let held = MouseModifiers {
                    shift: mods.contains(Mods::SHIFT),
                    alt: mods.contains(Mods::ALT),
                    ctrl: mods.contains(Mods::CTRL),
                };
                if to_program {
                    let bytes = match (kind, pressed) {
                        (PointerKind::Down, Some(b)) => encode_mouse_press(cell.0, cell.1, b, modes, held),
                        (PointerKind::Up, Some(b)) => encode_mouse_release(cell.0, cell.1, b, modes, held),
                        (PointerKind::Move, _) => encode_mouse_move(cell.0, cell.1, held_button, modes, held),
                        _ => Vec::new(),
                    };
                    s.write(&bytes);
                    return;
                }
                let mut engine = s.engine();
                let (pos, side): (Pos, Side) = engine.term.hit_test(row, col);
                match kind {
                    PointerKind::Down if pressed == Some(TerminalMouseButton::Left) => {
                        let kind = match clicks {
                            _ if mods.contains(Mods::ALT) => SelectionKind::Block,
                            2 => SelectionKind::Semantic,
                            3.. => SelectionKind::Lines,
                            _ => SelectionKind::Simple,
                        };
                        engine.term.selection_clear();
                        engine.term.selection_start(pos, side, kind);
                    }
                    PointerKind::Move | PointerKind::Up if selects => engine.term.selection_update(pos, side),
                    _ => return,
                }
                if let Some((clipboard, text)) = copies.zip(engine.term.selection_text()) {
                    clipboard.write_text(&text);
                }
                drop(engine);
                crate::widget::repaint(tree, id);
            });
        });
    }
    // A click the program took is not the app's either: its menu listens for clicks.
    let s = session.clone();
    cx.tree.on_pointer(id, PointerKind::Click, move |e| {
        if s.engine().term.mouse_reporting().is_some() && !e.mods.contains(Mods::SHIFT) {
            e.stop();
        }
    });
    let (writer, s) = (cx.handle.clone(), session.clone());
    let notches = Rc::new(Cell::new(0.0f32));
    cx.tree.on_pointer(id, PointerKind::Wheel, move |e| {
        e.stop();
        // Rows of 40 px (the tree's line of a wheel) times the setting, what is less kept for
        // the next turn.
        let set = settings();
        let total = notches.get() + e.delta.y / 40.0 * set.scroll.clamp(0.1, 10.0);
        let rows = total.trunc();
        notches.set(total - rows);
        if rows == 0.0 {
            return;
        }
        let (at, mods, s) = (e.local, e.mods, s.clone());
        writer.with(move |tree| wheel(tree, id, &s, (at, mods, set.alternate_scroll), rows as i32));
    });
}

/// `rows` up (positive) or down: to the program, as arrows, or through the history.
fn wheel(tree: &mut Tree, id: NodeId, s: &Session, (at, mods, arrows): (Point, Mods, bool), rows: i32) {
    let Some((row, col)) = grid_at(tree, id, at) else { return };
    let (modes, reports) = {
        let engine = s.engine();
        (engine.term.mode_snapshot(), engine.term.mouse_reporting().is_some())
    };
    let held =
        MouseModifiers { shift: false, alt: mods.contains(Mods::ALT), ctrl: mods.contains(Mods::CTRL) };
    if reports && !mods.contains(Mods::SHIFT) {
        let one = encode_wheel_event(
            row.max(0.0) as usize,
            col.max(0.0) as usize,
            rows.signum() as f64,
            modes,
            held,
        );
        s.write(&one.repeat(rows.unsigned_abs() as usize));
    } else if modes.alt_screen && (modes.alternate_scroll || arrows) {
        let arrow = KeySpec::Named(if rows > 0 { NamedKey::ArrowUp } else { NamedKey::ArrowDown });
        let one = s.engine().term.encode_key(&arrow, KeyMods::default()).unwrap_or_default();
        send(tree, id, s, &one.repeat(rows.unsigned_abs() as usize));
    } else {
        s.engine().term.scroll_viewport(-rows);
        crate::widget::repaint(tree, id);
    }
}
