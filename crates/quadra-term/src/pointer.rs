//! The terminal and the pointer (specs/18, "Input"): with mouse reporting on and Shift not
//! held, presses, drags and the wheel go to the program; otherwise a press and a drag select
//! (two clicks a word, three the line, Alt a block) and the wheel scrolls the history, or
//! sends arrows on the alternate screen when the program asked for alternate scroll.

use std::cell::Cell;
use std::rc::Rc;

use oneterm_vt::input::{
    KeyMods, KeySpec, MouseModifiers, NamedKey, TerminalMouseButton, encode_mouse_move, encode_mouse_press,
    encode_mouse_release, encode_wheel_event,
};
use oneterm_vt::{Pos, SelectionKind, Side};
use quadra_platform::{Mods, MouseButton};
use quadra_prim::Point;
use quadra_tree::{NodeId, PointerEvent, PointerKind, Tree};
use quadra_view::MountCx;

use crate::session::Session;
use crate::view::send;
use crate::widget::{PADDING, TerminalWidget};

/// Where a point of the node is in the grid: a row and a column, as fractions.
fn grid_at(tree: &mut Tree, id: NodeId, at: Point) -> Option<(f32, f32)> {
    let w = tree.widget_mut::<TerminalWidget>(id)?;
    Some(((at.y - PADDING) / w.line, (at.x - PADDING) / w.cell))
}

fn button(b: MouseButton) -> Option<TerminalMouseButton> {
    match b {
        MouseButton::Left => Some(TerminalMouseButton::Left),
        MouseButton::Middle => Some(TerminalMouseButton::Middle),
        MouseButton::Right => Some(TerminalMouseButton::Right),
        _ => None,
    }
}

pub(crate) fn pointer(cx: &mut MountCx, id: NodeId, session: &Session) {
    // The button held while the program reports, and whether a selection is being dragged.
    let reported: Rc<Cell<Option<TerminalMouseButton>>> = Rc::new(Cell::new(None));
    let selecting = Rc::new(Cell::new(false));
    for kind in [PointerKind::Down, PointerKind::Move, PointerKind::Up] {
        let (writer, s) = (cx.handle.clone(), session.clone());
        let (reported, selecting) = (reported.clone(), selecting.clone());
        cx.tree.on_pointer(id, kind, move |e: &mut PointerEvent| {
            let (at, mods, clicks, pressed) = (e.local, e.mods, e.clicks, button(e.button));
            let reports = s.engine().term.mouse_reporting().is_some() && !mods.contains(Mods::SHIFT);
            match kind {
                PointerKind::Down if pressed.is_none() => return,
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
            let s = s.clone();
            writer.with(move |tree| {
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
                drop(engine);
                crate::widget::repaint(tree, id);
            });
        });
    }
    let (writer, s) = (cx.handle.clone(), session.clone());
    let notches = Rc::new(Cell::new(0.0f32));
    cx.tree.on_pointer(id, PointerKind::Wheel, move |e| {
        e.stop();
        // Rows of 40 px (the tree's line of a wheel), what is less kept for the next turn.
        let total = notches.get() + e.delta.y / 40.0;
        let rows = total.trunc();
        notches.set(total - rows);
        if rows == 0.0 {
            return;
        }
        let (at, mods, s) = (e.local, e.mods, s.clone());
        writer.with(move |tree| wheel(tree, id, &s, (at, mods), rows as i32));
    });
}

/// `rows` up (positive) or down: to the program, as arrows, or through the history.
fn wheel(tree: &mut Tree, id: NodeId, s: &Session, (at, mods): (Point, Mods), rows: i32) {
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
    } else if modes.alt_screen && modes.alternate_scroll {
        let arrow = KeySpec::Named(if rows > 0 { NamedKey::ArrowUp } else { NamedKey::ArrowDown });
        let one = s.engine().term.encode_key(&arrow, KeyMods::default()).unwrap_or_default();
        send(tree, id, s, &one.repeat(rows.unsigned_abs() as usize));
    } else {
        s.engine().term.scroll_viewport(-rows);
        crate::widget::repaint(tree, id);
    }
}
