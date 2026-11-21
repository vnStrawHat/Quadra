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
}

const NAMES: [(&str, Level); 11] = [
    ("trace", Level::Debug),
    ("debug", Level::Debug),
    ("info", Level::Info),
    ("warn", Level::Warn),
    ("warning", Level::Warn),
    ("error", Level::Error),
    ("err", Level::Error),
    ("fatal", Level::Error),
    ("critical", Level::Error),
    ("crit", Level::Error),
    ("panic", Level::Error),
];

impl Level {
    /// The first level a line names: in capitals (`WARN`), as its first word in any case
    /// (`error: ...`), or in any case after `level`, `lvl` or `severity` (`level=warn`,
    /// `"level":"error"`); a line that starts as klog's (`E1121 ...`) by its letter.
    pub(crate) fn of(line: &str) -> Level {
        let said = plain(line);
        if let Some(level) = said.split_whitespace().next().and_then(klog) {
            return level;
        }
        // The first word is told by its place, as a word after `level` is.
        let mut told = true;
        for word in said.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()) {
            let named = NAMES.iter().find(|(name, _)| name.eq_ignore_ascii_case(word));
            if let Some(&(_, level)) = named.filter(|_| told || word.bytes().all(|b| b.is_ascii_uppercase()))
            {
                return level;
            }
            told = ["level", "lvl", "severity"].iter().any(|key| key.eq_ignore_ascii_case(word));
        }
        Level::None
    }
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
        "E" | "F" => Some(Level::Error),
        _ => None,
    }
}
