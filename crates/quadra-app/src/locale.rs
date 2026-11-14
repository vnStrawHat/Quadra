//! Formats by language (specs/15, "Language"): numbers, sizes of files, dates, times, times
//! relative to now, and the words of the calendar. `vi` and `en` are known; any other
//! language is formatted as `en`.

use crate::time::{Date, Time};

/// A language, by its tag: `vi`, `en`, `en-GB`. What decides formats and plurals is the
/// language, the part before the first `-`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Locale(pub String);

/// The words and marks of a language.
struct Words {
    months: [&'static str; 12],
    /// Two letters a day, Monday first: the heads of a calendar's columns.
    days: [&'static str; 7],
    /// Thousands, then the decimal point.
    marks: (char, char),
    /// The day the week begins with, Monday being 0.
    first_day: u8,
}

const VI: Words = Words {
    months: [
        "Tháng 1",
        "Tháng 2",
        "Tháng 3",
        "Tháng 4",
        "Tháng 5",
        "Tháng 6",
        "Tháng 7",
        "Tháng 8",
        "Tháng 9",
        "Tháng 10",
        "Tháng 11",
        "Tháng 12",
    ],
    days: ["T2", "T3", "T4", "T5", "T6", "T7", "CN"],
    marks: ('.', ','),
    first_day: 0,
};

const EN: Words = Words {
    months: [
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
    ],
    days: ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"],
    marks: (',', '.'),
    first_day: 6,
};

impl Locale {
    pub fn new(tag: &str) -> Locale {
        Locale(tag.to_string())
    }

    pub fn vi() -> Locale {
        Locale::new("vi")
    }

    pub fn en() -> Locale {
        Locale::new("en")
    }

    /// The language: the tag up to its first `-`, in lower case.
    pub fn language(&self) -> String {
        self.0.split(['-', '_']).next().unwrap_or_default().to_lowercase()
    }

    fn is_vi(&self) -> bool {
        self.language() == "vi"
    }

    fn words(&self) -> &'static Words {
        if self.is_vi() { &VI } else { &EN }
    }

    /// The form of a plural `n` takes: `one` or `other`. Vietnamese has one form.
    pub fn plural(&self, n: f64) -> &'static str {
        if !self.is_vi() && n == 1.0 { "one" } else { "other" }
    }

    /// A number with `decimals` places, its thousands grouped: `1.234,5`, `1,234.5`.
    pub fn number(&self, value: f64, decimals: usize) -> String {
        let (group, point) = self.words().marks;
        let text = format!("{:.*}", decimals, value.abs());
        let (whole, part) = text.split_once('.').unwrap_or((&text, ""));
        let mut out = String::new();
        for (i, digit) in whole.chars().enumerate() {
            if i > 0 && (whole.len() - i) % 3 == 0 {
                out.push(group);
            }
            out.push(digit);
        }
        if !part.is_empty() {
            out.push(point);
            out.push_str(part);
        }
        let negative = value < 0.0 && out.chars().any(|c| c.is_ascii_digit() && c != '0');
        if negative { format!("-{out}") } else { out }
    }

    /// A size of a file, by 1024: `512 B`, `1,5 MB`, `1.5 MB`.
    pub fn size(&self, bytes: u64) -> String {
        const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
        let (mut value, mut unit) = (bytes as f64, 0);
        while value >= 1024.0 && unit < UNITS.len() - 1 {
            (value, unit) = (value / 1024.0, unit + 1);
        }
        let decimals = if unit == 0 || value >= 100.0 { 0 } else { 1 };
        let number = self.number(value, decimals);
        let number =
            number.strip_suffix(&format!("{}0", self.words().marks.1)).unwrap_or(&number).to_string();
        format!("{number} {}", UNITS[unit])
    }

    /// 14/11/2026; 11/14/2026.
    pub fn date(&self, date: Date) -> String {
        let Date { year, month, day } = date;
        if self.is_vi() { format!("{day:02}/{month:02}/{year}") } else { format!("{month}/{day}/{year}") }
    }

    /// 14 tháng 11 năm 2026; November 14, 2026.
    pub fn date_long(&self, date: Date) -> String {
        let Date { year, month, day } = date;
        match self.is_vi() {
            true => format!("{day} tháng {month} năm {year}"),
            false => format!("{} {day}, {year}", self.month(month)),
        }
    }

    /// 15:05; 3:05 PM.
    pub fn time(&self, time: Time) -> String {
        let Time { hour, minute, .. } = time;
        match self.is_vi() {
            true => format!("{hour:02}:{minute:02}"),
            false => {
                let half = if hour < 12 { "AM" } else { "PM" };
                format!("{}:{minute:02} {half}", (hour + 11) % 12 + 1)
            }
        }
    }

    /// How long ago, or how long until: `seconds` from now, before it negative. Under a
    /// minute is now.
    pub fn relative(&self, seconds: i64) -> String {
        let far = seconds.unsigned_abs();
        let (n, vi, en) = match far {
            0..60 => return if self.is_vi() { "vừa xong".into() } else { "just now".into() },
            60..3600 => (far / 60, "phút", "minute"),
            3600..86_400 => (far / 3600, "giờ", "hour"),
            86_400..2_592_000 => (far / 86_400, "ngày", "day"),
            2_592_000..31_536_000 => (far / 2_592_000, "tháng", "month"),
            _ => (far / 31_536_000, "năm", "year"),
        };
        match (self.is_vi(), seconds < 0) {
            (true, true) => format!("{n} {vi} trước"),
            (true, false) => format!("{n} {vi} nữa"),
            (false, past) => {
                let unit = if n == 1 { en.to_string() } else { format!("{en}s") };
                if past { format!("{n} {unit} ago") } else { format!("in {n} {unit}") }
            }
        }
    }

    /// The name of a month, from 1.
    pub fn month(&self, month: u8) -> &'static str {
        self.words().months[(month.clamp(1, 12) - 1) as usize]
    }

    /// The two letters of a day of the week, Monday being 0.
    pub fn day(&self, weekday: u8) -> &'static str {
        self.words().days[(weekday % 7) as usize]
    }

    /// The day weeks begin with, Monday being 0: Monday in Vietnamese, Sunday in English.
    pub fn first_day(&self) -> u8 {
        self.words().first_day
    }
}
