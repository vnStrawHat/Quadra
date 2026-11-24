//! Where things are in the editor (behaviour/18, "View" and "Soft wrap"): rows, the lines as
//! they show. Without wrap a line is a row; with it a line is broken at the text's width,
//! after the last blank that fits (a word longer than a row breaks inside it, blanks hang
//! past the edge), and rows drive every y: paint, the pointer, the caret, up and down.

use std::ops::Range;

mod wrap;

use quadra_prim::{Point, Rect};

use super::EditorWidget;
pub(super) use wrap::Rows;

impl EditorWidget {
    /// How many columns a row holds.
    fn cols(&self) -> usize {
        ((self.text_width() / self.cell).floor() as usize).max(1)
    }

    /// Brings the rows up to the text, the width and the tab size: the lines that changed
    /// are wrapped again, or all of them when the width or the tab size did.
    pub(super) fn sync_rows(&mut self) {
        if !self.wrap {
            self.rows = None;
            return;
        }
        let key = (self.doc.version, self.cols(), self.doc.tab_size.max(1));
        let rows = match self.rows.take() {
            Some(rows) if rows.key == key => Some(rows),
            Some(rows) => rows.update(&self.doc, key.1, key.2),
            None => None,
        };
        self.rows = Some(rows.unwrap_or_else(|| Rows::build(&self.doc, key.1, key.2)));
    }

    /// How many rows the text takes: its lines, or what they wrap into.
    pub fn row_count(&self) -> usize {
        self.rows.as_ref().map_or(self.doc.buffer.line_count(), |rows| rows.starts.len())
    }

    pub(super) fn line_rows(&self, line: usize) -> Range<usize> {
        match &self.rows {
            Some(rows) => rows.first[line] as usize..rows.first[line + 1] as usize,
            None => line..line + 1,
        }
    }

    pub(super) fn row_line(&self, row: usize) -> usize {
        match &self.rows {
            Some(rows) => rows.first.partition_point(|&first| first as usize <= row) - 1,
            None => row,
        }
    }

    /// Where a row starts and ends in the text; a row that is not its line's last ends where
    /// the next starts.
    pub fn row_span(&self, row: usize) -> Range<usize> {
        let line = self.row_line(row);
        let start = self.doc.buffer.line_start(line);
        match &self.rows {
            Some(rows) => {
                let end = if row + 1 < self.line_rows(line).end {
                    start + rows.starts[row + 1] as usize
                } else {
                    self.doc.buffer.line_end(line)
                };
                start + rows.starts[row] as usize..end
            }
            None => start..self.doc.buffer.line_end(line),
        }
    }

    /// The row an offset shows in, and its x from the left of the text, before scrolling.
    pub(super) fn place(&self, offset: usize) -> (usize, f32) {
        let rows = self.line_rows(self.doc.buffer.line_of(offset));
        let row = (rows.start..rows.end)
            .rev()
            .find(|&row| self.row_span(row).start <= offset)
            .unwrap_or(rows.start);
        let from = if self.wrap { self.doc.col_of(self.row_span(row).start) } else { 0 };
        (row, (self.doc.col_of(offset) - from) as f32 * self.cell)
    }

    /// The offset in a row nearest a column of it: in the row, not at the start of the next.
    pub(super) fn offset_in_row(&self, row: usize, col: usize) -> usize {
        let (line, span) = (self.row_line(row), self.row_span(row));
        let from = if self.wrap { self.doc.col_of(span.start) } else { 0 };
        let offset = self.doc.offset_at_col(line, from + col);
        if offset >= span.end && row + 1 < self.line_rows(line).end {
            let text = self.doc.buffer.slice(span.clone());
            return span.start + text.char_indices().last().map_or(0, |(i, _)| i);
        }
        offset.clamp(span.start, span.end)
    }

    /// The caret of the primary cursor, from the node's corner.
    pub fn caret_rect(&self) -> Rect {
        let (row, x) = self.place(self.doc.primary().head);
        Rect::new(self.gutter + x - self.scroll_x, row as f32 * self.line_px, 2.0, self.line_px)
    }

    /// The offset nearest a point from the node's corner.
    pub fn offset_at(&self, at: Point) -> usize {
        let row = ((at.y / self.line_px).floor().max(0.0) as usize).min(self.row_count() - 1);
        self.offset_in_row(row, self.col_at(at.x))
    }

    /// The column of its row nearest an x from the node's corner.
    pub(super) fn col_at(&self, x: f32) -> usize {
        ((x - self.gutter + self.scroll_x) / self.cell).round().max(0.0) as usize
    }

    /// Moves every cursor `by` rows, in the column it came from.
    pub(super) fn move_rows(&mut self, by: isize, extend: bool) {
        let last = self.row_count() - 1;
        for i in 0..self.doc.cursors.len() {
            let c = self.doc.cursors[i];
            let (row, x) = self.place(c.head);
            let goal = c.goal.unwrap_or((x / self.cell).round() as usize);
            let to = row.saturating_add_signed(by).min(last);
            let head = self.offset_in_row(to, goal);
            let anchor = if extend { c.anchor } else { head };
            self.doc.cursors[i] = crate::doc::Cursor { anchor, head, goal: Some(goal) };
        }
        self.doc.normalize();
        self.doc.break_group();
    }

    /// Scrolls sideways as little as keeps the caret in view, four cells from the edges.
    pub fn follow_caret(&mut self) {
        if self.wrap {
            self.scroll_x = 0.0;
            return;
        }
        let (_, x) = self.place(self.doc.primary().head);
        let (room, margin) = (self.text_width(), 4.0 * self.cell);
        self.widest = self.widest.max(x + margin);
        if x < self.scroll_x + margin {
            self.scroll_x = (x - margin).max(0.0);
        } else if x > self.scroll_x + room - margin {
            self.scroll_x = x - room + margin;
        }
    }

    /// Scrolls sideways by `dx`, as far as the widest line painted; never when it wraps.
    pub fn scroll_by(&mut self, dx: f32) -> bool {
        let end = if self.wrap { 0.0 } else { (self.widest - self.text_width()).max(0.0) };
        let to = (self.scroll_x + dx).clamp(0.0, end);
        std::mem::replace(&mut self.scroll_x, to) != to
    }
}
