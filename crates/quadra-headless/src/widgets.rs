//! The components by the names of their widgets (specs/17, "Widget names"): each is one of
//! the behaviours of this crate with what makes it that widget, its role to a reader or
//! whether it is a switch, so a look cannot give a radio group the role of a tab list.

use quadra_reactive::Signal;
use quadra_tree::Role;
use quadra_view::{Div, IntoProp, Region, View};

use crate::press::Press;
use crate::{choice, disclose, press};

pub use crate::combo::combo as combobox;
pub use crate::hint::{card as hover_card, tooltip};

/// A button on the face the look built (`press::press`).
pub fn button(face: Div) -> Press {
    press::press(face)
}

/// A checkbox on its face: a click on it (its label in it), `Confirm` or a reader turns `on`.
pub fn checkbox<M>(on: Signal<bool>, face: Div, disabled: impl IntoProp<bool, M>) -> Div {
    press::toggle(on, face, false, disabled)
}

/// A switch on its face: as a checkbox, a switch to a reader.
pub fn switch<M>(on: Signal<bool>, face: Div, disabled: impl IntoProp<bool, M>) -> Div {
    press::toggle(on, face, true, disabled)
}

/// The box around `count` radios: one stop of Tab, the arrows go round.
pub fn radio_group(group: Div, count: usize, chosen: Signal<usize>) -> Div {
    choice::group(group, Role::Group, count, chosen)
}

/// The radio at `i`: checked while `chosen` holds it.
pub fn radio(face: Div, i: usize, chosen: Signal<usize>) -> Div {
    choice::option(face, Role::Radio, i, chosen)
}

/// The strip of `count` tabs: one stop of Tab, the arrows go round.
pub fn tabs(strip: Div, count: usize, chosen: Signal<usize>) -> Div {
    choice::group(strip, Role::TabList, count, chosen)
}

/// The tab at `i`: selected while `chosen` holds it.
pub fn tab(face: Div, i: usize, chosen: Signal<usize>) -> Div {
    choice::option(face, Role::Tab, i, chosen)
}

/// The head of a section of an accordion: a click or `Confirm` opens and closes it.
pub fn accordion_head(face: Div, open: Signal<bool>) -> Div {
    disclose::head(face, open)
}

/// What a section holds, mounted while it is open.
pub fn accordion_body<V: View + 'static>(open: Signal<bool>, body: impl Fn() -> V + 'static) -> Region {
    disclose::body(open, body)
}
