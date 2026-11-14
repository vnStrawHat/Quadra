//! Dates and times (specs/15, "Time where the user is"): days from 1970 both ways, across
//! leap years and centuries, weekdays, and a moment at an offset from UTC.

use quadra_app::time::{Date, DateTime, Time};

#[test]
fn days_count_from_1970_both_ways_and_weeks_begin_on_monday() {
    for (date, days) in [
        (Date::new(1970, 1, 1), 0),
        (Date::new(1969, 12, 31), -1),
        (Date::new(2000, 2, 29), 11_016),
        (Date::new(2026, 11, 14), 20_771),
        (Date::new(1600, 3, 1), -135_080),
    ] {
        assert_eq!(date.days_since_1970(), days, "{date}");
        assert_eq!(Date::from_days_since_1970(days), date);
    }
    // Every day of four centuries comes back as itself, one after another.
    let mut date = Date::new(1900, 1, 1);
    for days in -25_567..120_000 {
        assert_eq!(Date::from_days_since_1970(days), date);
        let last = date.day == Date::days_in(date.year, date.month);
        date = match (last, date.month) {
            (false, _) => Date::new(date.year, date.month, date.day + 1),
            (true, 12) => Date::new(date.year + 1, 1, 1),
            (true, month) => Date::new(date.year, month + 1, 1),
        };
    }
    assert_eq!((Date::new(2026, 11, 14).weekday(), Date::new(1970, 1, 1).weekday()), (5, 3)); // Saturday, Thursday
    assert_eq!(Date::new(2024, 12, 30).add_days(3), Date::new(2025, 1, 2));
    assert_eq!(Date::days_in(1900, 2), 28);
}

#[test]
fn a_moment_is_the_date_and_time_at_its_offset() {
    // 2026-11-14 08:30:05 UTC is 15:30 in Hà Nội, and the day before in Los Angeles.
    let at =
        DateTime { date: Date::new(2026, 11, 14), time: Time { hour: 8, minute: 30, second: 5 } }.to_unix(0);
    let hanoi = DateTime::from_unix(at, 7 * 3600);
    assert_eq!(hanoi.to_string(), "2026-11-14 15:30:05");
    assert_eq!(DateTime::from_unix(at, -16 * 3600).to_string(), "2026-11-13 16:30:05");
    assert_eq!(hanoi.to_unix(7 * 3600), at);
    assert_eq!(DateTime::from_unix(-1, 0).to_string(), "1969-12-31 23:59:59");
}
