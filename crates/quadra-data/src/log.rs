//! A log (specs/13, "Data"): lines an app appends, packed in one buffer in the order of
//! their times, and which of them a filter lists.

use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::rc::Rc;

use quadra_reactive::{Signal, signal};

mod level;
mod listing;
mod store;
#[cfg(test)]
mod tests;

pub use level::Level;
use listing::{Listing, bit, matcher};
pub(crate) use store::Store;

struct Inner {
    store: RefCell<Store>,
    listing: RefCell<Listing>,
    /// How many lines are listed, and how many there are.
    listed: Signal<usize>,
    total: Signal<usize>,
    /// Written when the lines that are listed are other ones: a filter, a clearing, a line
    /// that came between others.
    changed: Signal<u32>,
    /// The filter's pattern is not one.
    wrong: Signal<bool>,
    /// The sources lines came from, a bit each (`source % 64`); whether a line has a level.
    sources: Signal<u64>,
    leveled: Signal<bool>,
    /// The most lines it holds; 0: no limit.
    limit: Cell<usize>,
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
        Log(Rc::new(Inner {
            store: RefCell::default(),
            listing: RefCell::new(Listing::new()),
            listed: signal(0),
            total: signal(0),
            changed: signal(0),
            wrong: signal(false),
            sources: signal(0),
            leveled: signal(false),
            limit: Cell::new(0),
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
        self.append_from(0, lines);
    }

    /// Adds lines of one source (a stream: a container, a pod), each after the last line
    /// whose time is not later, so that streams that come interleaved read in time order.
    pub fn append_from<T: AsRef<str>>(&self, source: u8, lines: impl IntoIterator<Item = (i64, T)>) {
        let (mut store, mut listing) = (self.0.store.borrow_mut(), self.0.listing.borrow_mut());
        let mut between = false;
        for (at_ms, text) in lines {
            let text = text.as_ref();
            let Some(i) = store.push(at_ms, text, source) else { continue };
            between |= i + 1 < store.len();
            if listing.indexed() {
                // The places after it move one down; it is listed if it matches.
                let after = listing.index.partition_point(|&k| (k as usize) < i);
                listing.index[after..].iter_mut().for_each(|k| *k += 1);
                if listing.accepts(text, store.level(i)) {
                    listing.index.insert(after, i as u32);
                }
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
        let leveled = store.leveled > 0;
        drop((store, listing));
        self.0.total.set(total);
        self.0.listed.set(listed);
        self.0.sources.update(|seen| *seen |= 1 << (source % 64));
        self.0.leveled.set(leveled);
        if trimmed || between {
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
        self.0.sources.set(0);
        self.0.leveled.set(false);
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

    /// How many sources its lines came from; a signal read.
    pub fn sources(&self) -> usize {
        self.0.sources.get().count_ones() as usize
    }

    /// Whether a line it holds has a level; a signal read.
    pub fn has_levels(&self) -> bool {
        self.0.leveled.get()
    }

    /// Lists the lines that say `query`, whatever their case; with `pattern` it is a regular
    /// expression. With `dim` every line is listed, and those that do not match are faint.
    /// False: the pattern is not one, and nothing is filtered. Every line is read once.
    pub fn set_filter(&self, query: &str, pattern: bool, dim: bool) -> bool {
        let built = matcher(query, pattern);
        let good = built.is_ok();
        let mut listing = self.0.listing.borrow_mut();
        (listing.matcher, listing.dim) = (built.unwrap_or(None), dim);
        drop(listing);
        self.relist();
        self.0.wrong.set(!good);
        good
    }

    /// Lists only the lines of `least` or graver (`Level::Warn`: warnings, errors and fatal
    /// ones), with the text filter; `Level::None` lists every level.
    pub fn set_level(&self, least: Level) {
        let graver = [Level::Debug, Level::Info, Level::Warn, Level::Error, Level::Fatal];
        let lit: Vec<Level> = graver.into_iter().filter(|&level| level >= least).collect();
        self.set_levels(if least == Level::None { &[] } else { &lit });
    }

    /// Lists only the lines of these levels, fatal counted as error, with the text filter;
    /// none lists every level.
    pub fn set_levels(&self, levels: &[Level]) {
        self.0.listing.borrow_mut().levels = levels.iter().fold(0, |set, &level| set | bit(level));
        self.relist();
    }

    /// Reads every line once against the filter.
    fn relist(&self) {
        let (store, mut listing) = (self.0.store.borrow(), self.0.listing.borrow_mut());
        let index: Vec<u32> = match listing.indexed() {
            true => (0..store.len())
                .filter(|&i| listing.accepts(store.line(i), store.level(i)))
                .map(|i| i as u32)
                .collect(),
            false => Vec::new(),
        };
        listing.index = index;
        let listed = if listing.indexed() { listing.index.len() } else { store.len() };
        drop((store, listing));
        self.0.listed.set(listed);
        self.0.changed.update(|n| *n += 1);
    }

    /// The line at a place of the listing, in the store.
    fn at(&self, place: usize) -> Option<usize> {
        let (store, listing) = (self.0.store.borrow(), self.0.listing.borrow());
        let i = if listing.indexed() { *listing.index.get(place)? as usize } else { place };
        (i < store.len()).then_some(i)
    }

    /// The line at a place of the listing: its number, when it was written, what it holds,
    /// and whether it is one the filter makes faint.
    pub fn line(&self, place: usize) -> Option<(usize, i64, String, bool)> {
        let i = self.at(place)?;
        let (store, listing) = (self.0.store.borrow(), self.0.listing.borrow());
        let text = store.line(i);
        Some((i, store.time(i), text.to_string(), listing.dim && !listing.accepts(text, store.level(i))))
    }

    /// The level of the line at a place of the listing.
    pub fn level(&self, place: usize) -> Level {
        self.at(place).map_or(Level::None, |i| self.0.store.borrow().level(i))
    }

    /// The source of the line at a place of the listing.
    pub fn source(&self, place: usize) -> u8 {
        self.at(place).map_or(0, |i| self.0.store.borrow().source(i))
    }

    /// Where the filter's text or pattern is in what the line at a place says, without its
    /// escape sequences: byte ranges of that.
    pub fn marks(&self, place: usize) -> Vec<Range<usize>> {
        let Some(i) = self.at(place) else { return Vec::new() };
        self.0.listing.borrow().marks(self.0.store.borrow().line(i))
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
