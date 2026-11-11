//! Dates (specs/behaviour/09, "Forms"): the month calendar, and the date picker that shows
//! one in a popup. A date is a year, a month and a day; the names are English until the
//! app's language says otherwise.

use std::fmt;

use quadra_layout::{Align, Justify};
use quadra_platform::Cursor;
use quadra_reactive::{Signal, signal};
use quadra_tree::{NodeId, Role, State};
use quadra_ui::{Div, MountCx, NodeRef, View, column, div, dynamic, row, text};

use crate::action::{CONTROL, Confirm};
use crate::button::icon_button;
use crate::icon::icon;
use crate::popup::popover;
use crate::{icons, size, token};

const WEEKDAYS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// A day of the calendar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    /// From 1 to 12.
    pub month: u8,
    pub day: u8,
}

impl Date {
    pub fn new(year: i32, month: u8, day: u8) -> Date {
        Date { year, month, day }
    }

    /// How many days a month has.
    pub fn days_in(year: i32, month: u8) -> u8 {
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        match month {
            2 => 28 + leap as u8,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        }
    }

    /// The day of the week, Monday being 0.
    pub fn weekday(self) -> u8 {
        const SHIFT: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let year = self.year - (self.month < 3) as i32;
        let sunday_first = year + year / 4 - year / 100 + year / 400 + SHIFT[self.month as usize - 1];
        (sunday_first + self.day as i32 + 6).rem_euclid(7) as u8
    }
}

impl fmt::Display for Date {
    /// Day, month, year: 05/03/2026.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}/{:02}/{}", self.day, self.month, self.year)
    }
}

/// The calendar of the month that `value` is in, and of the others by its arrows. A click on
/// a day writes it to `value` and runs `picked`.
fn month(value: Signal<Date>, picked: impl Fn() + Clone + 'static) -> Div {
    let at = value.get_untracked();
    // The month that shows: the arrows change it, and not the date.
    let shown = signal((at.year, at.month));
    let turn = move |by: i32| {
        shown.update(|(year, month)| {
            let months = *year * 12 + *month as i32 - 1 + by;
            (*year, *month) = (months.div_euclid(12), months.rem_euclid(12) as u8 + 1);
        })
    };
    let name = text(move || {
        let (year, month) = shown.get();
        format!("{} {year}", MONTHS[month as usize - 1])
    });
    let head = row().align_items(Align::Center);
    let head = head.child(icon_button(icons::CHEVRON_LEFT, "Previous month").on_click(move || turn(-1)));
    let head = head.child(row().grow(1.0).justify(Justify::Center).child(name));
    let head = head.child(icon_button(icons::CHEVRON_RIGHT, "Next month").on_click(move || turn(1)));

    let cell = || row().align_items(Align::Center).justify(Justify::Center).shrink(0.0).width(32.0);
    let names = WEEKDAYS.map(|name| cell().height(22.0).child(text(name).size(size::TEXT_SM.size)));
    let names = row().gap(2.0).text_color(token::TEXT_2).children(names);
    // The days in weeks of seven, the first week begun where its first day falls.
    let days = dynamic(move || {
        let (year, month) = shown.get();
        let before = Date::new(year, month, 1).weekday() as usize;
        let mut cells: Vec<Div> = (0..before).map(|_| cell().height(28.0)).collect();
        for day in 1..=Date::days_in(year, month) {
            let (date, picked) = (Date::new(year, month, day), picked.clone());
            let on = cell().height(28.0).radius(size::RADIUS_MD).cursor(Cursor::Pointer);
            let on = on.role(Role::Button).label(date.to_string()).selected(move || value.get() == date);
            let on = on.hover(|s| s.bg(token::PANEL_HOVER));
            let on = on.when(State::SELECTED, |s| s.bg(token::ACCENT_TINT).text_color(token::ACCENT_TEXT));
            let on = on.on_click(move |_| {
                value.set(date);
                picked();
            });
            cells.push(on.child(text(day.to_string())));
        }
        let mut weeks = column().gap(2.0);
        let mut cells = cells.into_iter().peekable();
        while cells.peek().is_some() {
            weeks = weeks.child(row().gap(2.0).children(cells.by_ref().take(7).collect::<Vec<_>>()));
        }
        weeks
    });
    column().gap(8.0).padding(8.0).child(head).child(names).child(days)
}

/// A month calendar in the page: `value` holds the chosen day, and a click on a day writes
/// it.
pub fn calendar(value: Signal<Date>) -> Div {
    let page = month(value, || {}).bg(token::OVERLAY_BG).radius(size::RADIUS_XL);
    page.border(1.0).border_color(token::LINE).elevation(1).shrink(0.0)
}

pub struct DatePicker {
    value: Signal<Date>,
}

/// A field of 180 px that shows a date, and opens a calendar to choose another.
pub fn date_picker(value: Signal<Date>) -> DatePicker {
    DatePicker { value }
}

impl View for DatePicker {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (value, open, opener) = (self.value, signal(false), NodeRef::new());
        let face = row().align_items(Align::Center).gap(6.0).shrink(0.0).width(180.0).height(size::H_MD);
        let face = face.padding_x(8.0).radius(size::RADIUS_MD).border(1.0).border_color(token::LINE);
        let face = face.hover(|s| s.bg(token::PANEL_HOVER).border_color(token::LINE_STRONG));
        let face = face.focus_visible(|s| s.border_color(token::BORDER_FOCUSED));
        let face = face.cursor(Cursor::Pointer).role(Role::Button).label(move || value.get().to_string());
        let face = face.expanded(open).focusable().key_context(CONTROL).node_ref(&opener);
        let face = face.on_click(move |_| open.update(|open| *open = !*open));
        let face = face.on_action::<Confirm>(move |_| open.set(true));
        let face = face.child(icon(icons::CALENDAR).size(14.0).color(token::TEXT_2));
        let face = face.child(text(move || value.get().to_string()));
        // A day chosen closes the calendar.
        let days = popover(open, &opener, move || month(value, move || open.set(false)));
        div().shrink(0.0).child(face).child(days).mount(cx)
    }
}
