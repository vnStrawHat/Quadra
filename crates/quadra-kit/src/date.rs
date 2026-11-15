//! Dates (specs/behaviour/09, "Forms"): the month calendar, and the date picker that shows
//! one in a popup. A date is a year, a month and a day; the names of months and days, the
//! first day of the week and how a date is written are those of the app's language.

use quadra_app::i18n::language;
use quadra_app::time::local_now;
use quadra_base::{disclose, time};
use quadra_layout::{Align, Justify};
use quadra_platform::{Cursor, Key};
use quadra_reactive::{Signal, signal};
use quadra_tree::{NodeId, State};
use quadra_ui::{Div, MountCx, NodeRef, View, by_text, column, div, dynamic, row, text};

use crate::button::icon_button;
use crate::icon::icon;
use crate::popup::popover;
use crate::{icons, size, token};

pub use quadra_app::time::Date;

/// The calendar of the month that `value` is in, and of the others by its arrows. A click on
/// a day writes it to `value` and runs `picked`; so does Enter on the days, which the arrows
/// of the keyboard move over. With `focused` the days take the focus when they show.
fn month(value: Signal<Date>, picked: impl Fn() + Clone + 'static, focused: bool) -> Div {
    let (at, lang, today) = (value.get_untracked(), language(), local_now().date);
    // The month that shows: the arrows change it, and not the date.
    let shown = signal((at.year, at.month));
    let turn = move |by: i32| {
        shown.update(|(year, month)| {
            let months = *year * 12 + *month as i32 - 1 + by;
            (*year, *month) = (months.div_euclid(12), months.rem_euclid(12) as u8 + 1);
        })
    };
    let words = lang.clone();
    let name = text(move || {
        let (year, month) = shown.get();
        format!("{} {year}", words.locale().month(month))
    });
    let head = row().align_items(Align::Center);
    let head = head.child(icon_button(icons::CHEVRON_LEFT, "Previous month").on_click(move || turn(-1)));
    let head = head.child(row().grow(1.0).justify(Justify::Center).child(name));
    let head = head.child(icon_button(icons::CHEVRON_RIGHT, "Next month").on_click(move || turn(1)));

    let cell = || row().align_items(Align::Center).justify(Justify::Center).shrink(0.0).width(32.0);
    // The days of the week from the one the language begins it with.
    let heads = lang.clone();
    let names = dynamic(move || {
        let words = heads.locale();
        let first = words.first_day();
        let name = |i: u8| {
            cell().height(by_text(22.0)).child(text(words.day((first + i) % 7)).size(size::TEXT_SM.size))
        };
        row().gap(2.0).text_color(token::TEXT_2).children((0..7).map(name).collect::<Vec<_>>())
    });
    // The days in weeks of seven, the first week begun where its first day falls.
    let take = picked.clone();
    let days = dynamic(move || {
        let (year, month) = shown.get();
        let words = lang.locale();
        let first = words.first_day();
        let before = ((Date::new(year, month, 1).weekday() + 7 - first) % 7) as usize;
        let mut cells: Vec<Div> = (0..before).map(|_| cell().height(by_text(28.0))).collect();
        for day in 1..=Date::days_in(year, month) {
            let (date, picked) = (Date::new(year, month, day), picked.clone());
            let on = cell().height(by_text(28.0)).radius(token::RADIUS_MD).cursor(Cursor::Pointer);
            let on = on.hover(|s| s.bg(token::PANEL_HOVER));
            let on = on.when(State::SELECTED, |s| s.bg(token::ACCENT_TINT).text_color(token::ACCENT_TEXT));
            let on = time::day(on, words.date(date), date, value, picked);
            // Today is in the accent, and stronger.
            let on = match date == today {
                true => on.text_color(token::ACCENT_TEXT).font_weight(size::WEIGHT_STRONG),
                false => on,
            };
            cells.push(on.child(text(day.to_string())));
        }
        let mut weeks = column().gap(2.0);
        let mut cells = cells.into_iter().peekable();
        while cells.peek().is_some() {
            weeks = weeks.child(row().gap(2.0).children(cells.by_ref().take(7).collect::<Vec<_>>()));
        }
        weeks
    });
    // The arrows move the day by one and by a week, the pages by a month; the month that
    // shows follows the day. Enter takes it.
    let grid = column().gap(2.0).focusable().radius(token::RADIUS_MD);
    let grid = grid.focus_visible(|s| s.outline(token::BORDER_FOCUSED));
    let grid = grid.on_key_down(move |e| {
        let date = value.get_untracked();
        let to = match e.key {
            Key::Left => date.add_days(-1),
            Key::Right => date.add_days(1),
            Key::Up => date.add_days(-7),
            Key::Down => date.add_days(7),
            Key::PageUp | Key::PageDown => {
                let by = if e.key == Key::PageUp { -1 } else { 1 };
                let months = date.year * 12 + date.month as i32 - 1 + by;
                let (year, month) = (months.div_euclid(12), months.rem_euclid(12) as u8 + 1);
                Date::new(year, month, date.day.min(Date::days_in(year, month)))
            }
            Key::Enter => {
                e.stop();
                return take();
            }
            _ => return,
        };
        e.stop();
        value.set(to);
        shown.set((to.year, to.month));
    });
    let grid = match focused {
        true => grid.on_mount(|tree, id| tree.after_layout(move |tree| tree.focus(id))),
        false => grid,
    };
    // The names of the days are the first row of the weeks: 2 px over the next.
    column().gap(8.0).padding(8.0).child(head).child(grid.child(names).child(days))
}

/// A month calendar in the page: `value` holds the chosen day, and a click on a day writes
/// it.
pub fn calendar(value: Signal<Date>) -> Div {
    let page = month(value, || {}, false).bg(token::OVERLAY_BG).radius(token::RADIUS_XL);
    page.border(1.0).border_color(token::LINE_STRONG).elevation(1).shrink(0.0)
}

pub struct DatePicker {
    value: Signal<Date>,
    test_id: Option<&'static str>,
}

/// A field of 180 px that shows a date, and opens a calendar to choose another.
pub fn date_picker(value: Signal<Date>) -> DatePicker {
    DatePicker { value, test_id: None }
}

impl DatePicker {
    pub fn test_id(mut self, id: &'static str) -> Self {
        self.test_id = Some(id);
        self
    }
}

impl View for DatePicker {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (value, open, opener) = (self.value, signal(false), NodeRef::new());
        let (lang, said) = (language(), language());
        let face =
            row().align_items(Align::Center).gap(6.0).shrink(0.0).width(180.0).height(by_text(size::H_MD));
        let face = face.padding_x(8.0).radius(token::RADIUS_MD).border(1.0).border_color(token::LINE_STRONG);
        let face = face.hover(|s| s.bg(token::PANEL_HOVER).border_color(token::LINE_STRONG));
        let face = face.focus_visible(|s| s.border_color(token::BORDER_FOCUSED));
        let face = face.cursor(Cursor::Pointer).label(move || lang.locale().date(value.get()));
        let face = match self.test_id {
            Some(id) => face.test_id(id),
            None => face,
        };
        // A button that opens the calendar: while it is open the focus is in it.
        let face = disclose::head(face.node_ref(&opener), open);
        let face = face.child(icon(icons::CALENDAR).size(14.0).color(token::TEXT_2));
        let face = face.child(text(move || said.locale().date(value.get())));
        // A day chosen closes the calendar.
        let days = popover(open, &opener, move || month(value, move || open.set(false), true));
        div().shrink(0.0).child(face).child(days).mount(cx)
    }
}
