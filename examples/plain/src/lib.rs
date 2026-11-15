//! A second kit (specs/17, "The second kit"): square and flat, with outlines, in ink on paper.
//! What each control does is `quadra-headless`'s; this crate is only the look, made of boxes and
//! texts, with no code of `quadra-design`.

mod popups;

pub use popups::{Act, dialog, select};

use quadra::{Align, Color, Cursor, Div, Paint, Refine, Role, Signal, div, memo, row, text};
use quadra_headless::{choice, list, press, slider};

/// The colours of the look: ink, paper, a grey under the pointer, a darker one while pressed.
pub const INK: Color = Color::rgb(0x1A1A1A);
pub const PAPER: Color = Color::rgb(0xFFFFFF);
pub const HOVER: Color = Color::rgb(0xEEEEEE);
pub const PRESSED: Color = Color::rgb(0xDDDDDD);
/// The height of a control.
pub const HIGH: f32 = 28.0;

/// The focus shows as a second line outside the box.
fn ring(s: Refine) -> Refine {
    s.outline(INK)
}

/// The root of a window: ink on paper, in a column that fills it.
pub fn root() -> Div {
    quadra::column().grow(1.0).bg(PAPER).text_color(INK).text_size(14.0).padding(24.0).gap(16.0)
}

/// A box with a line of ink around it.
pub fn framed() -> Div {
    row().align_items(Align::Center).border(1.0).border_color(INK).bg(PAPER)
}

/// A button that does `f`.
pub fn button(label: &str, f: impl FnMut() + 'static) -> Div {
    let face = framed().height(HIGH).padding_x(12.0).cursor(Cursor::Pointer).label(label);
    let face = face.hover(|s| s.bg(HOVER)).active(|s| s.bg(PRESSED)).focus_visible(ring);
    press::press(face.child(text(label.to_string()))).on_click(f).build()
}

/// A square filled while `on`, `size` wide.
fn mark(size: f32, on: impl Fn() -> bool + 'static) -> Div {
    let fill = move || -> Paint { if on() { INK } else { PAPER }.into() };
    div().size(size, size).shrink(0.0).border(1.0).border_color(INK).bg(fill)
}

/// A checkbox: a square filled while on, and its label.
pub fn checkbox(on: Signal<bool>, label: &str) -> Div {
    let face = row().gap(8.0).align_items(Align::Center).cursor(Cursor::Pointer).label(label);
    let face = face.focus_visible(ring).child(mark(14.0, move || on.get())).child(text(label.to_string()));
    press::toggle(on, face, false, false)
}

/// A switch: a frame with a square at its left while off, at its right and filled while on.
pub fn switch(on: Signal<bool>, label: &str) -> Div {
    let knob =
        mark(10.0, move || on.get()).absolute().top(2.0).left(move || if on.get() { 16.0 } else { 2.0 });
    let track = div().size(30.0, 16.0).shrink(0.0).border(1.0).border_color(INK).child(knob);
    let face = row().gap(8.0).align_items(Align::Center).cursor(Cursor::Pointer).label(label);
    press::toggle(on, face.focus_visible(ring).child(track).child(text(label.to_string())), true, false)
}

/// One of a few options, as radios in a column: a square filled for the one chosen.
pub fn radio_group(options: &[&str], chosen: Signal<usize>) -> Div {
    let group =
        choice::group(quadra::column().gap(6.0).focus_visible(ring), Role::Group, options.len(), chosen);
    group.children(options.iter().enumerate().map(|(i, name)| {
        let face = row().gap(8.0).align_items(Align::Center).cursor(Cursor::Pointer).label(*name);
        let face = face.child(mark(12.0, move || chosen.get() == i)).child(text(name.to_string()));
        choice::option(face, Role::Radio, i, chosen)
    }))
}

/// Tabs in a row: the chosen one stands on a line of ink 2 px thick.
pub fn tabs(names: &[&str], chosen: Signal<usize>) -> Div {
    let strip = row().border_bottom(1.0).border_color(INK).focus_visible(ring);
    let strip = choice::group(strip, Role::TabList, names.len(), chosen);
    strip.children(names.iter().enumerate().map(|(i, name)| {
        let face = row().height(HIGH).padding_x(12.0).align_items(Align::Center).cursor(Cursor::Pointer);
        let face = face.border_bottom(move || if chosen.get() == i { 3.0 } else { 0.0 }).border_color(INK);
        let face = face.hover(|s| s.bg(HOVER)).label(*name).child(text(name.to_string()));
        choice::option(face, Role::Tab, i, chosen)
    }))
}

/// A slider of `value` from `min` to `max` by `step`, 160 px wide: a line, and a square on it.
pub fn slider(value: Signal<f32>, (min, max): (f32, f32), step: f32) -> Div {
    let (knob, wide) = (12.0, 160.0);
    let scale = slider::Scale::new((min, max), step, knob / 2.0, wide - knob);
    let line = div().absolute().left(knob / 2.0).top(HIGH / 2.0).size(wide - knob, 1.0).bg(INK);
    let at = move || scale.part(value.get()) * scale.track;
    let square = div().absolute().top((HIGH - knob) / 2.0).left(at).size(knob, knob).bg(INK);
    let face = div().size(wide, HIGH).cursor(Cursor::Pointer).focus_visible(ring).child(line).child(square);
    slider::slider(face, value, scale, memo(|| false))
}

/// A list of `items` in a frame, `height` tall: the selected row in ink, its text on paper.
pub fn list(items: Vec<String>, selected: Signal<Option<usize>>, height: f32) -> impl quadra::View {
    let count = items.len();
    let row_of = move |at: Signal<usize>| {
        let (items, on) = (items.clone(), move || selected.get() == Some(at.get()));
        let says = items.clone();
        let fill = move || -> Paint { if on() { INK } else { PAPER }.into() };
        let ink = move || -> Paint { if on() { PAPER } else { INK }.into() };
        let face = row().height(24.0).padding_x(8.0).align_items(Align::Center).bg(fill).text_color(ink);
        let face = face.label(move || says.get(at.get()).cloned().unwrap_or_default());
        face.child(text(move || items.get(at.get()).cloned().unwrap_or_default()))
    };
    let frame =
        quadra::column().width(200.0).height(height).border(1.0).border_color(INK).focus_visible(ring);
    list::list(move || count, selected, 24.0, row_of).frame(frame).empty(|| text("Nothing"))
}

/// A row of controls with a word before them.
pub fn labeled(word: &str, control: impl quadra::View + 'static) -> Div {
    row()
        .gap(12.0)
        .align_items(Align::Center)
        .child(div().width(80.0).child(text(word.to_string())))
        .child(control)
}
