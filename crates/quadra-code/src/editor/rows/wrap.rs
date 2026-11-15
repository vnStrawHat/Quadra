//! The rows of a wrapped text, kept as the text changes: the lines a change was are wrapped
//! again, and the rows of the others move.

use std::ops::Range;

use quadra_text::cell_step;

use crate::doc::Doc;

/// The rows of a wrapped text, as of a version of it, a count of columns and a tab size.
pub(in crate::editor) struct Rows {
    pub key: (u64, usize, usize),
    /// The first row of each line, and the count of rows after the last.
    pub first: Vec<u32>,
    /// Where each row starts, as an offset in its line.
    pub starts: Vec<u32>,
}

/// Adds the starts of the rows of a line to `out`: after the last blank that fits, inside
/// a word longer than a row; blanks hang past the edge.
fn wrap_line(text: &str, cols: usize, tab: usize, out: &mut Vec<u32>) {
    out.push(0);
    let (mut at, mut col, mut row_start, mut row_col) = (0, 0, 0, 0);
    let mut blank: Option<(usize, usize)> = None; // after the last blank of the row
    while at < text.len() {
        let (end, width) = cell_step(text, at);
        let byte = text.as_bytes()[at];
        let next = if byte == b'\t' { (col / tab + 1) * tab } else { col + width };
        let is_blank = byte == b' ' || byte == b'\t';
        if !is_blank && next - row_col > cols && at > row_start {
            let (b, c) = blank.filter(|&(b, _)| b > row_start).unwrap_or((at, col));
            out.push(b as u32);
            (row_start, row_col, blank) = (b, c, None);
            if b < at {
                continue; // what is left of the word may not fit either
            }
        }
        if is_blank {
            blank = Some((end, next));
        }
        (col, at) = (next, end);
    }
}

/// The first row of each of `lines` from the first of them, and the starts of their rows.
fn wrap(doc: &Doc, lines: Range<usize>, cols: usize, tab: usize) -> (Vec<u32>, Vec<u32>) {
    let (mut first, mut starts) = (Vec::with_capacity(lines.len()), Vec::new());
    for line in lines {
        first.push(starts.len() as u32);
        wrap_line(&doc.buffer.line(line), cols, tab, &mut starts);
    }
    (first, starts)
}

impl Rows {
    pub fn build(doc: &Doc, cols: usize, tab: usize) -> Rows {
        let (mut first, starts) = wrap(doc, 0..doc.buffer.line_count(), cols, tab);
        first.push(starts.len() as u32);
        Rows { key: (doc.version, cols, tab), first, starts }
    }

    /// The rows as of the document's version: the lines its changes since were are
    /// wrapped again. `None` when the changes are no longer known, or the columns or the
    /// tab size are others.
    pub fn update(mut self, doc: &Doc, cols: usize, tab: usize) -> Option<Rows> {
        if (self.key.1, self.key.2) != (cols, tab) {
            return None;
        }
        // The changes in turn, each a line a row for now; what is to wrap again is kept as
        // ranges of lines of the text as it is after them.
        let mut stale: Vec<Range<usize>> = Vec::new();
        for change in doc.line_changes_since(self.key.0)? {
            let (from, to, by) =
                (change.line, change.line + change.was, change.now as isize - change.was as isize);
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
            self.splice(from..to, (0..change.now as u32).collect(), vec![0; change.now]);
        }
        for range in stale {
            let (first, starts) = wrap(doc, range.clone(), cols, tab);
            self.splice(range, first, starts);
        }
        self.key.0 = doc.version;
        Some(self)
    }

    /// Puts lines whose first rows are `first` (from 0) and whose rows start at `starts` in
    /// place of the rows of `lines`; the rows after them move.
    fn splice(&mut self, lines: Range<usize>, first: Vec<u32>, starts: Vec<u32>) {
        let rows = self.first[lines.start] as usize..self.first[lines.end] as usize;
        let by = starts.len() as i64 - rows.len() as i64;
        let (base, count) = (rows.start as u32, first.len());
        self.starts.splice(rows, starts);
        self.first.splice(lines.clone(), first.into_iter().map(|row| base + row));
        for row in &mut self.first[lines.start + count..] {
            *row = (*row as i64 + by) as u32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::{Cursor, Edit};

    #[test]
    fn rows_kept_through_changes_are_the_rows_built_again() {
        let words = ["a", "bb", "ccc dddd", "\n", "eeeee ffffff", "\n\n", "\t", " "];
        let mut text = String::new();
        for i in 0..400 {
            text.push_str(words[i * 7 % words.len()]);
        }
        let mut doc = Doc::new(&text);
        let mut rows = Rows::build(&doc, 9, 4);
        let mut seed = 7u64;
        let mut next = |n: usize| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as usize % n.max(1)
        };
        for step in 0..300 {
            // One edit or several (cursors), now and then undone.
            let len = doc.buffer.len();
            let mut at: Vec<usize> = (0..1 + next(3)).map(|_| next(len + 1)).collect();
            at.sort_unstable();
            at.dedup();
            let text = doc.buffer.text();
            let edits = at
                .iter()
                .filter(|&&a| text.is_char_boundary(a))
                .map(|&a| Edit { range: a..a, text: words[next(words.len())].into(), caret: None })
                .collect::<Vec<_>>();
            doc.apply(edits, false);
            if next(4) == 0 {
                doc.undo();
            }
            doc.cursors = vec![Cursor::default()];
            if next(3) == 0 {
                rows = rows.update(&doc, 9, 4).expect("the changes are kept");
                let whole = Rows::build(&doc, 9, 4);
                assert_eq!((&rows.first, &rows.starts), (&whole.first, &whole.starts), "{step}");
            }
        }
    }
}
