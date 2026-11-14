//! Time where the user is (specs/15, "Time where the user is"): days of the proleptic
//! Gregorian calendar, times of a day, and a moment as a date and a time at an offset from
//! UTC.

use std::fmt;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use quadra_platform::Platform;
use quadra_reactive::use_context;

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
        // 1970-01-01 was a Thursday.
        (self.days_since_1970() + 3).rem_euclid(7) as u8
    }

    /// Days from 1970-01-01, before it negative.
    pub fn days_since_1970(self) -> i64 {
        // The year from March, so the leap day is its last.
        let (month, day) = (self.month as i64, self.day as i64);
        let year = self.year as i64 - (month <= 2) as i64;
        let era = year.div_euclid(400);
        let of_era = year - era * 400;
        let of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
        let of_era_days = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
        era * 146_097 + of_era_days - 719_468
    }

    /// The day so many days from 1970-01-01.
    pub fn from_days_since_1970(days: i64) -> Date {
        let days = days + 719_468;
        let era = days.div_euclid(146_097);
        let of_era = days - era * 146_097;
        let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
        let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_from_march = (5 * of_year + 2) / 153;
        let day = (of_year - (153 * month_from_march + 2) / 5 + 1) as u8;
        let month = if month_from_march < 10 { month_from_march + 3 } else { month_from_march - 9 } as u8;
        let year = (year_of_era + era * 400 + (month <= 2) as i64) as i32;
        Date { year, month, day }
    }

    /// The day `days` after this one, or before it.
    pub fn add_days(self, days: i64) -> Date {
        Date::from_days_since_1970(self.days_since_1970() + days)
    }
}

impl fmt::Display for Date {
    /// Year, month, day: 2026-03-05, the form logs and files use. A user is shown a date in
    /// the form of their language (`Locale::date`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// A time of a day.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

/// A moment, as the calendar and the clock say it at an offset from UTC.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime {
    pub date: Date,
    pub time: Time,
}

impl DateTime {
    /// The moment `seconds` after 1970-01-01 00:00 UTC, where the clock is `offset` seconds
    /// ahead of UTC.
    pub fn from_unix(seconds: i64, offset: i32) -> DateTime {
        let local = seconds + offset as i64;
        let (days, of_day) = (local.div_euclid(86_400), local.rem_euclid(86_400));
        let (hour, minute, second) = ((of_day / 3600) as u8, (of_day / 60 % 60) as u8, (of_day % 60) as u8);
        DateTime { date: Date::from_days_since_1970(days), time: Time { hour, minute, second } }
    }

    /// Seconds after 1970-01-01 00:00 UTC, for a moment said at `offset`.
    pub fn to_unix(self, offset: i32) -> i64 {
        let Time { hour, minute, second } = self.time;
        let of_day = hour as i64 * 3600 + minute as i64 * 60 + second as i64;
        self.date.days_since_1970() * 86_400 + of_day - offset as i64
    }
}

impl fmt::Display for DateTime {
    /// 2026-03-05 14:07:09.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Time { hour, minute, second } = self.time;
        write!(f, "{} {hour:02}:{minute:02}:{second:02}", self.date)
    }
}

/// The date and time where the user is: the system's clock at the offset the app's platform
/// says. In UTC where no app is around.
pub fn local_now() -> DateTime {
    let now = unix_now();
    let offset = use_context::<Rc<dyn Platform>>().map_or(0, |platform| platform.utc_offset(now));
    DateTime::from_unix(now, offset)
}

/// Seconds since 1970-01-01 00:00 UTC by the system's clock.
pub fn unix_now() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(since) => since.as_secs() as i64,
        Err(before) => -(before.duration().as_secs() as i64),
    }
}
