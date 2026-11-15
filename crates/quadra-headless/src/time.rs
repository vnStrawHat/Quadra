//! Times and days (specs/17, "The components"): a time of the day, edited as parts that take
//! the focus each (the arrows step the part round its ends, Home and End go to them, digits
//! type it), and the days of a calendar, each chosen by a click. The look gives each part's
//! face and each day's. Dates themselves are the app's (`quadra_app::time::Date`).

use std::fmt;

use quadra_reactive::Signal;
use quadra_tree::Role;
use quadra_ui::Div;

use crate::action::{Decrease, Increase, MoveFirst, MoveLast, SLIDER};

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

/// A part after a digit was typed: the digit joins the one before it while that still makes
/// a value of the part (1 then 5 is 15), and stands alone otherwise.
pub fn typed(old: u8, digit: u8, limit: u8) -> u8 {
    let two = (old % 10) * 10 + digit;
    if old < 10 && two < limit { two } else { digit }
}

/// One part of a time on its face, `name` to a reader (a slider with its value): `read`
/// gives it, `write` takes it, it is less than `limit`, and an arrow steps it by `step`.
pub fn part(
    face: Div,
    name: &'static str,
    (limit, step): (u8, u8),
    read: impl Fn() -> u8 + Copy + 'static,
    write: impl Fn(u8) + Copy + 'static,
) -> Div {
    let by = move |change: i32| write((read() as i32 + change).rem_euclid(limit as i32) as u8);
    let part = face.role(Role::Slider).label(name.to_string());
    let part = part.value(move || format!("{:02}", read())).focusable().key_context(SLIDER);
    let part = part.on_action::<Increase>(move |_| by(step as i32));
    let part = part.on_action::<Decrease>(move |_| by(-(step as i32)));
    let part =
        part.on_action::<MoveFirst>(move |_| write(0)).on_action::<MoveLast>(move |_| write(limit - 1));
    part.on_text(move |typed_text| {
        for digit in typed_text.chars().filter_map(|c| c.to_digit(10)) {
            write(typed(read(), digit as u8, limit));
        }
    })
}

/// A day of a calendar on its face, named `name`: a button, `State::SELECTED` while `value`
/// holds it; a click writes it and runs `picked`.
pub fn day<T: Copy + PartialEq + 'static>(
    face: Div,
    name: String,
    date: T,
    value: Signal<T>,
    picked: impl Fn() + 'static,
) -> Div {
    let face = face.role(Role::Button).label(name).selected(move || value.get() == date);
    face.on_click(move |_| {
        value.set(date);
        picked();
    })
}
