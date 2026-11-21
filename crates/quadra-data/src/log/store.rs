//! The lines of a log, packed in one buffer.

/// Lines packed end to end: line `i` is the bytes from `ends[i - 1]` to `ends[i]`.
#[derive(Default)]
pub(crate) struct Store {
    bytes: String,
    ends: Vec<u32>,
    /// When each was written, in milliseconds since 1970; 0: it does not say.
    times: Vec<i64>,
    /// How grave each is, read as it came.
    levels: Vec<super::Level>,
    /// The bytes of the longest line yet: how far the view may scroll sideways.
    pub(super) widest: usize,
}

impl Store {
    pub fn len(&self) -> usize {
        self.ends.len()
    }

    /// Adds a line. Past 4 GB of text nothing is added.
    pub fn push(&mut self, at_ms: i64, text: &str) {
        if let Ok(end) = u32::try_from(self.bytes.len() + text.len()) {
            self.bytes.push_str(text);
            self.ends.push(end);
            self.times.push(at_ms);
            self.levels.push(super::Level::of(text));
            self.widest = self.widest.max(text.len());
        }
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
        self.levels.drain(..k);
    }

    pub fn line(&self, i: usize) -> &str {
        let from = if i == 0 { 0 } else { self.ends[i - 1] };
        &self.bytes[from as usize..self.ends[i] as usize]
    }

    pub fn time(&self, i: usize) -> i64 {
        self.times[i]
    }

    pub fn level(&self, i: usize) -> super::Level {
        self.levels[i]
    }
}
