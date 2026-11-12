//! What is done to whole lines: indent, delete, duplicate, move, comment. Each works on the
//! lines the cursors are in, and is one group for undo.

use std::ops::RangeInclusive;

use super::{Cursor, Doc, Edit};

impl Doc {
    /// The lines the cursors are in, as ranges that neither touch nor overlap. A selection
    /// that ends at the start of a line does not take that line.
    fn cursor_lines(&self) -> Vec<RangeInclusive<usize>> {
        let mut out: Vec<RangeInclusive<usize>> = Vec::new();
        for c in &self.cursors {
            let first = self.buffer.line_of(c.start());
            let mut last = self.buffer.line_of(c.end());
            if last > first && c.end() == self.buffer.line_start(last) {
                last -= 1;
            }
            match out.last_mut() {
                Some(range) if first <= range.end() + 1 => *range = *range.start()..=last.max(*range.end()),
                _ => out.push(first..=last),
            }
        }
        out
    }

    /// Adds a level of indent to every line of the cursors that is not empty, or takes one
    /// away.
    pub fn indent_lines(&mut self, add: bool) {
        let unit = self.indent_unit();
        let edit = |line: usize| {
            let start = self.buffer.line_start(line);
            if add {
                let empty = self.buffer.line_end(line) == start;
                return (!empty).then(|| Edit { range: start..start, text: unit.clone(), caret: None });
            }
            let lead = self.leading(line);
            let spaces = lead.bytes().take(self.tab_size).take_while(|byte| *byte == b' ').count();
            let less = if lead.starts_with('\t') { 1 } else { spaces };
            (less > 0).then(|| Edit { range: start..start + less, text: String::new(), caret: None })
        };
        let edits = self.cursor_lines().into_iter().flatten().filter_map(edit).collect();
        self.apply(edits, false);
    }

    /// Takes out every line of the cursors.
    pub fn delete_lines(&mut self) {
        let count = self.buffer.line_count();
        let edit = |lines: RangeInclusive<usize>| {
            let mut start = self.buffer.line_start(*lines.start());
            let end = if lines.end() + 1 < count {
                self.buffer.line_start(lines.end() + 1)
            } else {
                self.buffer.len()
            };
            // The last line goes with the line break before it.
            if lines.end() + 1 >= count {
                start = start.saturating_sub(1);
            }
            Edit { range: start..end, text: String::new(), caret: None }
        };
        let edits = self.cursor_lines().into_iter().map(edit).collect();
        self.apply(edits, false);
    }

    /// Puts a copy of the lines of the cursors under them.
    pub fn duplicate_lines(&mut self) {
        let edit = |lines: RangeInclusive<usize>| {
            let (start, end) = (self.buffer.line_start(*lines.start()), self.buffer.line_end(*lines.end()));
            Edit { range: end..end, text: format!("\n{}", self.buffer.slice(start..end)), caret: None }
        };
        let edits = self.cursor_lines().into_iter().map(edit).collect();
        self.apply(edits, false);
    }

    /// Moves the lines of the cursors a line up or down, and the cursors with them.
    pub fn move_lines(&mut self, down: bool) {
        let ranges = self.cursor_lines();
        let last = self.buffer.line_count() - 1;
        if (down && *ranges[ranges.len() - 1].end() >= last) || (!down && *ranges[0].start() == 0) {
            return;
        }
        // The line a block changes places with, and how far the block goes for it.
        let mut edits = Vec::with_capacity(ranges.len());
        let mut moved: Vec<(RangeInclusive<usize>, isize)> = Vec::with_capacity(ranges.len());
        for lines in ranges {
            let (first, end) = (*lines.start(), *lines.end());
            let block = self.buffer.slice(self.buffer.line_start(first)..self.buffer.line_end(end));
            let other = self.buffer.line(if down { end + 1 } else { first - 1 });
            let (range, text) = if down {
                (self.buffer.line_start(first)..self.buffer.line_end(end + 1), format!("{other}\n{block}"))
            } else {
                (self.buffer.line_start(first - 1)..self.buffer.line_end(end), format!("{block}\n{other}"))
            };
            let by = other.len() as isize + 1;
            moved.push((lines, if down { by } else { -by }));
            edits.push(Edit { range, text, caret: None });
        }
        // A cursor goes with the block its line is in; one that selects to the start of the
        // line after a block, with that block.
        let with = |c: &Cursor| {
            let line = self.buffer.line_of(c.head);
            let of = |(lines, _): &&(RangeInclusive<usize>, isize)| {
                (*lines.start()..=lines.end() + 1).contains(&line)
            };
            let by = moved.iter().find(of).map_or(0, |(_, by)| *by);
            Cursor {
                anchor: c.anchor.wrapping_add_signed(by),
                head: c.head.wrapping_add_signed(by),
                goal: None,
            }
        };
        let cursors: Vec<Cursor> = self.cursors.iter().map(with).collect();
        self.apply(edits, false);
        // A selection to the start of the line after a block that is now the last line ends
        // with the text.
        let len = self.buffer.len();
        self.cursors = cursors
            .iter()
            .map(|c| Cursor { anchor: c.anchor.min(len), head: c.head.min(len), goal: None })
            .collect();
        self.normalize();
        if let Some(group) = self.undo.last_mut() {
            group.after.clone_from(&self.cursors);
        }
    }

    /// Makes comments of the lines of the cursors, or takes the marks away when every line
    /// that is not blank has one.
    pub fn toggle_comment(&mut self) {
        let mark = format!("{} ", self.comment);
        let mut edits = Vec::new();
        for lines in self.cursor_lines() {
            // The lines that are not blank: where their text starts in them, and the text.
            let texts: Vec<(usize, usize, String)> = lines
                .map(|line| (line, self.buffer.line(line).into_owned()))
                .map(|(line, text)| (line, text.len() - text.trim_start_matches([' ', '\t']).len(), text))
                .filter(|(_, indent, text)| *indent < text.len())
                .collect();
            let all = texts.iter().all(|(_, indent, text)| text[*indent..].starts_with(&self.comment));
            let least = texts.iter().map(|(_, indent, _)| *indent).min().unwrap_or(0);
            for (line, indent, text) in &texts {
                let start = self.buffer.line_start(*line);
                edits.push(if all {
                    let marked =
                        if text[*indent..].starts_with(&mark) { mark.len() } else { self.comment.len() };
                    Edit { range: start + indent..start + indent + marked, text: String::new(), caret: None }
                } else {
                    Edit { range: start + least..start + least, text: mark.clone(), caret: None }
                });
            }
        }
        self.apply(edits, false);
    }
}
