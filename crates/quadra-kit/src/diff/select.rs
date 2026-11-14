//! What the user does to a diff besides reading it: rows selected and copied as a unified
//! diff writes them, F7 to the next run of changes, the wheel sideways.

use std::rc::Rc;

use quadra_base::action::{NextChange, PreviousChange};
use quadra_base::table::{Column, Sheet, sheet};
use quadra_platform::{Clipboard, Mods};
use quadra_reactive::{Memo, Signal, untrack};
use quadra_tree::{Paint, PointerKind, Role, State as On};
use quadra_ui::Div;

use super::Sides;
use super::rows::Row;
use super::view::as_text;
use crate::token;

pub(super) type Listed = Memo<Rc<Vec<Row>>>;

/// The rows as a table of one column: what each says is its text in a unified diff.
pub(super) fn sheet_of(sides: Signal<Rc<Sides>>, listed: Listed) -> Sheet {
    let column = Column { width: 0.0, least: 0.0, sortable: false, counts: false, restless: false };
    let count = Rc::new(move || listed.with(|rows| rows.len()));
    let says = move |row: usize, _| {
        let at = listed.with(|rows| rows.get(row).copied());
        at.map_or_else(String::new, |at| sides.with_untracked(|sides| as_text(sides, at)))
    };
    sheet(vec![column], count, Rc::new(says), None)
}

/// A row that a click selects: tinted when selected, outlined when the keys are on it.
pub(super) fn face(sheet: &Sheet, face: Div, place: Signal<usize>) -> Div {
    let mark = sheet.clone();
    let ring = move || -> Paint {
        let here = mark.focused.get() && mark.cursor.get() == Some(place.get());
        if here { token::BORDER_FOCUSED.into() } else { Paint::default() }
    };
    let face = face.outline(ring).when(On::SELECTED, |s| s.bg(token::ACCENT.alpha(0.12)));
    sheet.row(face, place)
}

/// The first row of the next run of changes after `from`, or of the one before it.
fn change(rows: &[Row], from: Option<usize>, back: bool) -> Option<usize> {
    let changed = |i: usize| !matches!(rows[i], Row::Same { .. } | Row::Fold { .. });
    let mut starts = (0..rows.len()).filter(|&i| changed(i) && (i == 0 || !changed(i - 1)));
    match (from, back) {
        (Some(from), true) => starts.rfind(|&i| i < from),
        (None, true) => starts.next_back(),
        (from, false) => starts.find(|&i| from.is_none_or(|from| i > from)),
    }
}

/// The frame of the rows: a table named "Diff" with the keys of a table, Ctrl-C, and F7.
pub(super) fn frame(sheet: &Sheet, frame: Div, listed: Listed, clipboard: Option<Rc<dyn Clipboard>>) -> Div {
    let go = |back: bool| {
        let sheet = sheet.clone();
        move || {
            let to = untrack(|| listed.with(|rows| change(rows, sheet.cursor.get(), back)));
            if let Some(to) = to {
                sheet.pick(to, Mods::NONE);
            }
        }
    };
    let (next, previous) = (go(false), go(true));
    let frame = sheet.frame(frame, clipboard).role(Role::Table).label("Diff".to_string());
    frame.on_action::<NextChange>(move |_| next()).on_action::<PreviousChange>(move |_| previous())
}

/// A row that the wheel scrolls sideways: turned so, or with Shift. `end` is how far, for
/// the width of the row.
pub(super) fn sideways(face: Div, sx: Signal<f32>, end: impl Fn(f32) -> f32 + 'static) -> Div {
    face.on_pointer(PointerKind::Wheel, move |e| {
        let dx = if e.mods.contains(Mods::SHIFT) { -e.delta.y } else { -e.delta.x };
        if dx != 0.0 {
            e.stop();
            let end = end(e.size.width).max(0.0);
            sx.update(|x| *x = (*x + dx).clamp(0.0, end));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keys_go_to_the_first_row_of_each_run_of_changes() {
        let same = Row::Same { old: 0, new: 0 };
        let (gone, came) = (Row::Removed { old: 1, with: None }, Row::Added { new: 1, with: None });
        let rows = [same, gone, came, same, same, came, same];
        assert_eq!(change(&rows, None, false), Some(1));
        assert_eq!(change(&rows, Some(1), false), Some(5));
        assert_eq!(change(&rows, Some(5), false), None);
        assert_eq!(change(&rows, Some(5), true), Some(1));
        assert_eq!(change(&rows, None, true), Some(5));
    }
}
