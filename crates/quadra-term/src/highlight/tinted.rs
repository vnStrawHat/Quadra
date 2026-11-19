//! The prompt whose command ended last, tinted by its status.

use std::ops::Range;

use oneterm_vt::SnapshotRow;

use super::scan::Role;
use super::{Highlight, Marks};

impl Marks {
    /// The rows of the prompt whose command ended last, and the class its sign takes: the
    /// prompt line above the newest one, success for a status of 0, error otherwise.
    pub(crate) fn tinted(&self, rows: &[SnapshotRow], status: i32) -> Option<(Range<usize>, u8)> {
        let prompt = |r: usize| self.rows.get(&rows[r].id).is_some_and(|e| e.role == Some(Role::Prompt));
        let (mut before, mut last, mut r) = (None, None, 0);
        while r < rows.len() {
            if !prompt(r) {
                r += 1;
                continue;
            }
            let start = r;
            while rows[r].wrapped && r + 1 < rows.len() && prompt(r + 1) {
                r += 1;
            }
            r += 1;
            before = last.replace(start..r);
        }
        let class = if status == 0 { Highlight::Success } else { Highlight::Error };
        before.map(|rows| (rows, class as u8))
    }
}
