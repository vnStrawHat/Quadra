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
    /// The first level a line names: in capitals (`WARN`), or in any case after `level`,
    /// `lvl` or `severity` (`level=warn`, `"level":"error"`).
    pub(crate) fn of(line: &str) -> Level {
        let said = plain(line);
        let mut told = false;
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
