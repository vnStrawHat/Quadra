//! The time input (specs/behaviour/09, "Forms"): hours and minutes as two parts that take the
//! focus each. The arrows step the part with the focus, round its ends; digits type it.

use quadra_base::time as base;
pub use quadra_base::time::TimeOfDay;
use quadra_layout::Align;
use quadra_reactive::Signal;
use quadra_tree::{NodeId, Role};
use quadra_ui::{Div, MountCx, View, by_text, div, row, text};

use crate::icon::icon;
use crate::{icons, size, token};

pub struct TimeInput {
    value: Signal<TimeOfDay>,
    step: u8,
}

/// A field of 120 px that edits a time.
pub fn time_input(value: Signal<TimeOfDay>) -> TimeInput {
    TimeInput { value, step: 1 }
}

impl TimeInput {
    /// How many minutes an arrow steps the minutes by.
    pub fn minute_step(mut self, minutes: u8) -> Self {
        self.step = minutes.max(1);
        self
    }
}

/// One part of the time: `read` gives it, `write` takes it, and it is less than `limit`.
fn part(
    name: &'static str,
    (limit, step): (u8, u8),
    read: impl Fn() -> u8 + Copy + 'static,
    write: impl Fn(u8) + Copy + 'static,
) -> Div {
    let face = div().padding_x(3.0).radius(token::RADIUS_SM).focus(|s| s.bg(token::PRESS));
    base::part(face, name, (limit, step), read, write).child(text(move || format!("{:02}", read())))
}

impl View for TimeInput {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (value, step) = (self.value, self.step);
        let hours = part(
            "Hours",
            (24, 1),
            move || value.get().hour,
            move |hour| value.update(|time| time.hour = hour),
        );
        let minutes = part(
            "Minutes",
            (60, step),
            move || value.get().minute,
            move |minute| value.update(|time| time.minute = minute),
        );
        let face =
            row().align_items(Align::Center).gap(6.0).shrink(0.0).width(120.0).height(by_text(size::H_MD));
        let face = face.padding_x(8.0).radius(token::RADIUS_MD).border(1.0).border_color(token::LINE_STRONG);
        let face = face.focus_within(|s| s.border_color(token::BORDER_FOCUSED));
        let face = face.role(Role::Group).label(move || value.get().to_string());
        let face = face.child(icon(icons::CLOCK).size(14.0).color(token::TEXT_2));
        face.child(row().child(hours).child(text(":")).child(minutes)).mount(cx)
    }
}
