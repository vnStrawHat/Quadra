//! A log (specs/13, "Data"): lines an app appends, packed in one buffer, and which of them
//! a filter lists.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_reactive::{Signal, signal};
use regex_lite::{Regex, RegexBuilder};

use crate::ansi::plain;

mod store;

pub(crate) use store::Store;

/// What a filter looks for in what a line says.
enum Matcher {
    All,
    /// A text, in lower case: found whatever the case of the line.
    Text(String),
    Pattern(Regex),
}

impl Matcher {
    fn matches(&self, line: &str) -> bool {
        match self {
            Matcher::All => true,
            // ponytail: a copy of the line in lower case for each test; a search that folds
            // case as it goes when a filter over millions of lines is too slow.
            Matcher::Text(query) => plain(line).to_lowercase().contains(query),
            Matcher::Pattern(pattern) => pattern.is_match(&plain(line)),
        }
    }
}

struct Listing {
    matcher: Matcher,
    /// The lines that do not match are listed too, faint.
    dim: bool,
    /// The lines that match, when only those are listed.
    index: Vec<u32>,
}

impl Listing {
    fn indexed(&self) -> bool {
        !self.dim && !matches!(self.matcher, Matcher::All)
    }
}

struct Inner {
    store: RefCell<Store>,
    listing: RefCell<Listing>,
    /// How many lines are listed, and how many there are.
    listed: Signal<usize>,
    total: Signal<usize>,
    /// Written when the lines that are listed are other ones: a filter, a clearing.
    changed: Signal<u32>,
    /// The filter's pattern is not one.
    wrong: Signal<bool>,
    /// The most lines it holds; 0: no limit.
    limit: std::cell::Cell<usize>,
}

/// A log an app appends to and a [`crate::log_view`] shows. A clone is the same log.
#[derive(Clone)]
pub struct Log(Rc<Inner>);

impl Default for Log {
    fn default() -> Log {
        Log::new()
    }
}

impl Log {
    pub fn new() -> Log {
        let listing = Listing { matcher: Matcher::All, dim: false, index: Vec::new() };
        Log(Rc::new(Inner {
            store: RefCell::default(),
            listing: RefCell::new(listing),
            listed: signal(0),
            total: signal(0),
            changed: signal(0),
            wrong: signal(false),
            limit: std::cell::Cell::new(0),
        }))
    }

    /// Holds at most `lines` lines: past that the oldest go, an eighth of the limit at a
    /// time, so that dropping them costs little a line.
    pub fn limit(self, lines: usize) -> Log {
        self.0.limit.set(lines);
        self
    }

    /// Adds lines, each with when it was written, in milliseconds since 1970 (0: no time).
    pub fn append<T: AsRef<str>>(&self, lines: impl IntoIterator<Item = (i64, T)>) {
        let (mut store, mut listing) = (self.0.store.borrow_mut(), self.0.listing.borrow_mut());
        for (at_ms, text) in lines {
            let (i, text) = (store.len() as u32, text.as_ref());
            store.push(at_ms, text);
            if listing.indexed() && listing.matcher.matches(text) {
                listing.index.push(i);
            }
        }
        let limit = self.0.limit.get();
        let over = store.len().saturating_sub(limit);
        let trimmed = limit > 0 && over > limit / 8;
        if trimmed {
            store.drop_first(over);
            listing.index.retain(|&i| i as usize >= over);
            listing.index.iter_mut().for_each(|i| *i -= over as u32);
        }
        let (total, listed) =
            (store.len(), if listing.indexed() { listing.index.len() } else { store.len() });
        drop((store, listing));
        self.0.total.set(total);
        self.0.listed.set(listed);
        if trimmed {
            self.0.changed.update(|n| *n += 1); // the lines at each place are other ones
        }
    }

    pub fn push(&self, at_ms: i64, text: &str) {
        self.append([(at_ms, text)]);
    }

    pub fn clear(&self) {
        *self.0.store.borrow_mut() = Store::default();
        self.0.listing.borrow_mut().index.clear();
        self.0.total.set(0);
        self.0.listed.set(0);
        self.0.changed.update(|n| *n += 1);
    }

    /// How many lines the log holds; a signal read.
    pub fn len(&self) -> usize {
        self.0.total.get()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// How many lines the filter lists; a signal read.
    pub fn listed(&self) -> usize {
        self.0.listed.get()
    }

    /// Lists the lines that say `query`, whatever their case; with `pattern` it is a regular
    /// expression. With `dim` every line is listed, and those that do not match are faint.
    /// False: the pattern is not one, and nothing is filtered. Every line is read once.
    pub fn set_filter(&self, query: &str, pattern: bool, dim: bool) -> bool {
        let built = match (query.is_empty(), pattern) {
            (true, _) => Ok(Matcher::All),
            (_, false) => Ok(Matcher::Text(query.to_lowercase())),
            (_, true) => RegexBuilder::new(query).case_insensitive(true).build().map(Matcher::Pattern),
        };
        let (store, good) = (self.0.store.borrow(), built.is_ok());
        let mut listing = Listing { matcher: built.unwrap_or(Matcher::All), dim, index: Vec::new() };
        if listing.indexed() {
            let matching = (0..store.len()).filter(|&i| listing.matcher.matches(store.line(i)));
            listing.index = matching.map(|i| i as u32).collect();
        }
        let listed = if listing.indexed() { listing.index.len() } else { store.len() };
        drop(store);
        *self.0.listing.borrow_mut() = listing;
        self.0.listed.set(listed);
        self.0.wrong.set(!good);
        self.0.changed.update(|n| *n += 1);
        good
    }

    /// The line at a place of the listing: its number, when it was written, what it holds,
    /// and whether it is one the filter makes faint.
    pub fn line(&self, place: usize) -> Option<(usize, i64, String, bool)> {
        let (store, listing) = (self.0.store.borrow(), self.0.listing.borrow());
        let i = if listing.indexed() { *listing.index.get(place)? as usize } else { place };
        let text = (i < store.len()).then(|| store.line(i))?;
        Some((i, store.time(i), text.to_string(), listing.dim && !listing.matcher.matches(text)))
    }

    /// The bytes of the longest line it has held.
    pub(crate) fn widest(&self) -> usize {
        self.0.store.borrow().widest
    }

    /// What the view follows: when the listed lines are other ones, and a pattern that is
    /// not one.
    pub(crate) fn signals(&self) -> (Signal<u32>, Signal<bool>) {
        (self.0.changed, self.0.wrong)
    }
}

/// Milliseconds since 1970 as the time of their day, in UTC: `12:04:05.123`.
pub fn clock(ms: i64) -> String {
    let day = ms.rem_euclid(86_400_000);
    format!("{:02}:{:02}:{:02}.{:03}", day / 3_600_000, day / 60_000 % 60, day / 1000 % 60, day % 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(n: usize) -> Vec<(i64, String)> {
        let entry = |i: usize| match i % 10 {
            3 => (i as i64 * 1000, format!("\u{1b}[31mERROR\u{1b}[0m request {i} failed")),
            _ => (i as i64 * 1000, format!("INFO request {i} ok")),
        };
        (0..n).map(entry).collect()
    }

    #[test]
    fn lines_are_packed_and_read_back() {
        let mut store = Store::default();
        for (at, text) in [(5, "one"), (6, ""), (7, "thrée")] {
            store.push(at, text);
        }
        assert_eq!((store.len(), store.line(0), store.line(1), store.line(2)), (3, "one", "", "thrée"));
        assert_eq!((store.time(2), clock(1_793_966_400_000 + 3_723_045)), (7, "13:02:03.045".to_string()));
    }

    #[test]
    fn a_filter_hides_or_dims_by_a_text_or_a_pattern() {
        let log = Log::new();
        log.append(entries(100));
        assert_eq!((log.len(), log.listed()), (100, 100));
        // Whatever the case, and not in the escape sequences.
        assert!(log.set_filter("error", false, false));
        assert_eq!((log.listed(), log.line(0).map(|line| line.0)), (10, Some(3)));
        assert!(log.set_filter("31m", false, false) && log.listed() == 0);
        // Lines that come are tested as they come.
        log.set_filter("error", false, false);
        log.append(entries(14));
        assert_eq!((log.len(), log.listed(), log.line(10).map(|line| line.0)), (114, 12, Some(103)));
        // A pattern: 15, 25 .. 95 (a digit before the 5).
        assert!(log.set_filter(r"request \d+5 ", true, false));
        assert_eq!(log.listed(), 9);
        // Dimmed, every line is listed, and those that do not match are faint.
        log.set_filter("error", false, true);
        assert_eq!((log.listed(), log.line(3).unwrap().3, log.line(4).unwrap().3), (114, false, true));
        // A pattern that is not one filters nothing.
        assert!(!log.set_filter("(", true, false));
        assert_eq!((log.listed(), log.line(500)), (114, None));
        log.clear();
        assert_eq!((log.len(), log.listed(), log.line(0)), (0, 0, None));
    }

    #[test]
    fn past_its_limit_the_oldest_lines_go_an_eighth_at_a_time() {
        let log = Log::new().limit(80);
        log.append(entries(90));
        assert_eq!(log.len(), 90); // ten over: not yet an eighth
        log.set_filter("error", false, false);
        log.push(90_000, "ERROR the last");
        // Eleven over: they go, and the filter's lines are where they were.
        assert_eq!((log.len(), log.line(0).map(|l| l.0)), (80, Some(2)));
        assert_eq!(log.line(0).map(|l| l.1), Some(13_000));
        assert_eq!(log.line(log.listed() - 1).map(|l| l.2), Some("ERROR the last".to_string()));
    }
}
