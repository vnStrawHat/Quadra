//! How grave a line of a log is, read from its words once, when it is appended.

use crate::ansi::plain;

/// How grave a line is: `None` when it does not say.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    #[default]
    None,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

const NAMES: [(&str, Level); 11] = [
    ("trace", Level::Debug),
    ("debug", Level::Debug),
    ("info", Level::Info),
    ("warn", Level::Warn),
    ("warning", Level::Warn),
    ("error", Level::Error),
    ("err", Level::Error),
    ("fatal", Level::Fatal),
    ("critical", Level::Fatal),
    ("crit", Level::Fatal),
    ("panic", Level::Fatal),
];

impl Level {
    /// Its word in a log's level column: `INFO`, `WARN`, `ERROR`, `FATAL`, `DEBUG`; none
    /// for `None`.
    pub fn word(self) -> &'static str {
        ["", "DEBUG", "INFO", "WARN", "ERROR", "FATAL"][self as usize]
    }

    /// The first level a line names: in capitals (`WARN`), as its first word in any case
    /// (`error: ...`), or in any case after `level`, `lvl` or `severity` (`level=warn`,
    /// `"level":"error"`); a line that starts as klog's (`E1121 ...`) by its letter. A JSON
    /// line's `level` field comes before the words of its message.
    pub(crate) fn of(line: &str) -> Level {
        let said = plain(line);
        if let Some(level) = said.split_whitespace().next().and_then(klog) {
            return level;
        }
        let keyed = said.trim_start().starts_with('{').then(|| named(&said, false)).flatten();
        keyed.or_else(|| named(&said, true)).unwrap_or(Level::None)
    }
}

/// The first level named after `level`, `lvl` or `severity`; with `loose`, also as the first
/// word, or in capitals anywhere.
fn named(said: &str, loose: bool) -> Option<Level> {
    // The first word is told by its place, as a word after `level` is.
    let mut told = loose;
    for word in said.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()) {
        let named = NAMES.iter().find(|(name, _)| name.eq_ignore_ascii_case(word));
        let capitals = loose && word.bytes().all(|b| b.is_ascii_uppercase());
        if let Some(&(_, level)) = named.filter(|_| told || capitals) {
            return Some(level);
        }
        told = ["level", "lvl", "severity"].iter().any(|key| key.eq_ignore_ascii_case(word));
    }
    None
}

/// klog's first word: the level's letter, then the month and the day (`E1121`).
fn klog(word: &str) -> Option<Level> {
    let (letter, day) = word.split_at_checked(1)?;
    if day.len() != 4 || !day.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    match letter {
        "I" => Some(Level::Info),
        "W" => Some(Level::Warn),
        "E" => Some(Level::Error),
        "F" => Some(Level::Fatal),
        _ => None,
    }
}
