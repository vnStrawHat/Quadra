//! The lines of a log, packed in one buffer, in the order of their times.

use super::Level;

/// Lines packed end to end: line `i` is the bytes from `ends[i - 1]` to `ends[i]`.
#[derive(Default)]
pub(crate) struct Store {
    bytes: String,
    ends: Vec<u32>,
    /// When each was written, in milliseconds since 1970; 0: it does not say.
    times: Vec<i64>,
    /// How grave each is, read as it came.
    levels: Vec<Level>,
    /// Where each came from: the app's number of a stream.
    sources: Vec<u8>,
    /// The bytes of the longest line yet: how far the view may scroll sideways.
    pub(super) widest: usize,
    /// How many lines have a level.
    pub(super) leveled: usize,
}

impl Store {
    pub fn len(&self) -> usize {
        self.ends.len()
    }

    /// Adds a line after the last one whose time is not later (a line with no time goes at
    /// the end), and says where. Past 4 GB of text nothing is added.
    // ponytail: a line far older than the newest moves the bytes after it; a store kept by
    // arrival with an order of its own when streams come hours apart.
    /// `level`: the app's, or none to read it from the words.
    pub fn push(&mut self, at_ms: i64, text: &str, source: u8, level: Option<Level>) -> Option<usize> {
        u32::try_from(self.bytes.len() + text.len()).ok()?;
        let mut i = self.len();
        while at_ms != 0 && i > 0 && self.times[i - 1] > at_ms {
            i -= 1;
        }
        let from = if i == 0 { 0 } else { self.ends[i - 1] };
        let grown = text.len() as u32;
        self.bytes.insert_str(from as usize, text);
        self.ends[i..].iter_mut().for_each(|end| *end += grown);
        self.ends.insert(i, from + grown);
        let level = level.unwrap_or_else(|| Level::of(text));
        self.times.insert(i, at_ms);
        self.levels.insert(i, level);
        self.sources.insert(i, source);
        self.widest = self.widest.max(text.len());
        self.leveled += usize::from(level != Level::None);
        Some(i)
    }

    /// Drops the first `k` lines.
    pub(super) fn drop_first(&mut self, k: usize) {
        let k = k.min(self.len());
        if k == 0 {
            return;
        }
        let cut = self.ends[k - 1];
        self.bytes.drain(..cut as usize);
        self.ends.drain(..k);
        self.ends.iter_mut().for_each(|end| *end -= cut);
        self.times.drain(..k);
        self.leveled -= self.levels.drain(..k).filter(|&level| level != Level::None).count();
        self.sources.drain(..k);
    }

    pub fn line(&self, i: usize) -> &str {
        let from = if i == 0 { 0 } else { self.ends[i - 1] };
        &self.bytes[from as usize..self.ends[i] as usize]
    }

    pub fn time(&self, i: usize) -> i64 {
        self.times[i]
    }

    pub fn level(&self, i: usize) -> Level {
        self.levels[i]
    }

    pub fn source(&self, i: usize) -> u8 {
        self.sources[i]
    }
}
