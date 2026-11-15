//! Which lines the changes of a document were (specs/13, "Code"): what a view keeps by line
//! (the rows of a wrapped text) is made again for those alone.

use std::collections::VecDeque;
use std::ops::Range;

use super::Doc;

/// The changes kept: a view further behind makes what it keeps again whole.
const KEPT: usize = 256;

/// A change of lines: the `was` lines from `line` are now `now` lines. Its line is as the
/// text was just before it; the changes of one version come in the order they were made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineChange {
    pub line: usize,
    pub was: usize,
    pub now: usize,
}

#[derive(Default)]
pub(super) struct Changes {
    /// Each with the version it led to.
    kept: VecDeque<(u64, LineChange)>,
    /// The newest version of a change no longer kept.
    lost: u64,
}

fn breaks(text: &str) -> usize {
    text.bytes().filter(|&byte| byte == b'\n').count()
}

impl Doc {
    /// Puts `text` in place of `removed`, which is at `at`, and notes the lines it changed.
    pub(super) fn put(&mut self, at: usize, removed: &str, text: &str) {
        let line = self.buffer.line_of(at);
        self.buffer.delete(at..at + removed.len());
        self.buffer.insert(at, text);
        let change = LineChange { line, was: breaks(removed) + 1, now: breaks(text) + 1 };
        let changes = &mut self.changes;
        changes.kept.push_back((self.version + 1, change));
        if changes.kept.len() > KEPT
            && let Some((version, _)) = changes.kept.pop_front()
        {
            changes.lost = version;
        }
    }

    /// The changes of lines since `version`, in order; `None` when they are no longer all
    /// known.
    pub fn line_changes_since(&self, version: u64) -> Option<impl Iterator<Item = LineChange> + '_> {
        let changes = &self.changes;
        (version >= changes.lost)
            .then(|| changes.kept.iter().filter(move |(v, _)| *v > version).map(|(_, c)| *c))
    }

    /// Takes what a view keeps a line each, as of `version`, to the lines of now: `splice`
    /// puts `now` lines not yet known in place of `lines`, a change at a time; what is given
    /// back is the lines of the text as it is now that are to be made again (in ranges
    /// apart). `None` when the changes are no longer all known.
    pub fn changed_lines(
        &self,
        version: u64,
        mut splice: impl FnMut(Range<usize>, usize),
    ) -> Option<Vec<Range<usize>>> {
        let mut stale: Vec<Range<usize>> = Vec::new();
        for change in self.line_changes_since(version)? {
            let (from, to) = (change.line, change.line + change.was);
            let by = change.now as isize - change.was as isize;
            let mut fresh = from..to;
            stale.retain_mut(|range| {
                if range.end < from || range.start > to {
                    if range.start > to {
                        *range = range.start.wrapping_add_signed(by)..range.end.wrapping_add_signed(by);
                    }
                    return true;
                }
                fresh = fresh.start.min(range.start)..fresh.end.max(range.end);
                false
            });
            stale.push(fresh.start..fresh.end.wrapping_add_signed(by));
            splice(from..to, change.now);
        }
        Some(stale)
    }
}
