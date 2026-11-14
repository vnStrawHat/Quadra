//! The time input (specs/behaviour/09, "Forms"): hours and minutes as two parts that take the
//! focus each. The arrows step the part with the focus, round its ends; digits type it.

use std::fmt;

use quadra_layout::Align;
use quadra_reactive::Signal;
use quadra_tree::{NodeId, Role};
use quadra_ui::{Div, MountCx, View, div, row, text};

use crate::action::{Decrease, Increase, MoveFirst, MoveLast, SLIDER};
use crate::icon::icon;
use crate::{icons, size, token};

/// A time of the day: the hour from 0 to 23, the minute from 0 to 59.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeOfDay {
    pub hour: u8,
    pub minute: u8,
}

impl TimeOfDay {
    pub fn new(hour: u8, minute: u8) -> TimeOfDay {
        TimeOfDay { hour, minute }
    }
}

impl fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

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

/// A part after a digit was typed: the digit joins the one before it while that still makes
/// a value of the part (1 then 5 is 15), and stands alone otherwise.
fn typed(old: u8, digit: u8, limit: u8) -> u8 {
    let two = (old % 10) * 10 + digit;
    if old < 10 && two < limit { two } else { digit }
}

/// One part of the time: `read` gives it, `write` takes it, and it is less than `limit`.
fn part(
    name: &'static str,
    (limit, step): (u8, u8),
    read: impl Fn() -> u8 + Copy + 'static,
    write: impl Fn(u8) + Copy + 'static,
) -> Div {
    let by = move |change: i32| write((read() as i32 + change).rem_euclid(limit as i32) as u8);
    let part = div().padding_x(3.0).radius(size::RADIUS_SM).role(Role::Slider).label(name.to_string());
    let part = part.value(move || format!("{:02}", read())).focusable().key_context(SLIDER);
    let part = part.focus(|s| s.bg(token::PRESS));
    let part = part.on_action::<Increase>(move |_| by(step as i32));
    let part = part.on_action::<Decrease>(move |_| by(-(step as i32)));
    let part =
        part.on_action::<MoveFirst>(move |_| write(0)).on_action::<MoveLast>(move |_| write(limit - 1));
    let part = part.on_text(move |typed_text| {
        for digit in typed_text.chars().filter_map(|c| c.to_digit(10)) {
            write(typed(read(), digit as u8, limit));
        }
    });
    part.child(text(move || format!("{:02}", read())))
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
        let face = row().align_items(Align::Center).gap(6.0).shrink(0.0).width(120.0).height(size::H_MD);
        let face = face.padding_x(8.0).radius(size::RADIUS_MD).border(1.0).border_color(token::LINE_STRONG);
        let face = face.focus_within(|s| s.border_color(token::BORDER_FOCUSED));
        let face = face.role(Role::Group).label(move || value.get().to_string());
        let face = face.child(icon(icons::CLOCK).size(14.0).color(token::TEXT_2));
        face.child(row().child(hours).child(text(":")).child(minutes)).mount(cx)
    }
}
