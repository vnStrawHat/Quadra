//! Rows of a listing in the mono face that are selected as a table's are and scrolled
//! sideways: what the diff and the log view share (specs/13).

use std::rc::Rc;

use quadra_headless::table::{Column, Sheet, sheet};
use quadra_platform::Mods;
use quadra_reactive::Signal;
use quadra_tree::{Paint, PointerKind, State as On};
use quadra_ui::Div;

use crate::token;

/// A table of one column: `count()` rows, `says(row)` what copying a row gives.
pub fn one_column(count: impl Fn() -> usize + 'static, says: impl Fn(usize) -> String + 'static) -> Sheet {
    let column = Column { width: 0.0, least: 0.0, sortable: false, counts: false, restless: false };
    sheet(vec![column], Rc::new(count), Rc::new(move |row, _| says(row)), None)
}

/// A row that a click selects: tinted when selected, outlined when the keys are on it.
pub fn selectable(sheet: &Sheet, face: Div, place: Signal<usize>) -> Div {
    let mark = sheet.clone();
    let ring = move || -> Paint {
        let here = mark.focused.get() && mark.cursor.get() == Some(place.get());
        if here { token::BORDER_FOCUSED.into() } else { Paint::default() }
    };
    let face = face.outline(ring).when(On::SELECTED, |s| s.bg(token::ACCENT.alpha(0.12)));
    sheet.row(face, place)
}

/// A row that the wheel scrolls sideways: turned so, or with Shift. `end` is how far, for
/// the width of the row.
pub fn sideways(face: Div, sx: Signal<f32>, end: impl Fn(f32) -> f32 + 'static) -> Div {
    face.on_pointer(PointerKind::Wheel, move |e| {
        let dx = if e.mods.contains(Mods::SHIFT) { -e.delta.y } else { -e.delta.x };
        if dx != 0.0 {
            e.stop();
            let end = end(e.size.width).max(0.0);
            sx.update(|x| *x = (*x + dx).clamp(0.0, end));
        }
    })
}
