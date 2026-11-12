//! The commands that type and delete: text, a new line with its indent, deleting by
//! grapheme and by word, the tab, and what copy, cut and paste do.

use super::{Doc, Edit};

impl Doc {
    /// A level of indent.
    pub(super) fn indent_unit(&self) -> String {
        if self.use_tabs { "\t".to_string() } else { " ".repeat(self.tab_size) }
    }

    /// Puts `text` at every cursor, in place of what it selects. `typing` joins it to the
    /// typing before it, for undo; a paste is a group of its own.
    pub fn insert_text(&mut self, text: &str, typing: bool) {
        let edits = self.cursors.iter().map(|c| Edit::typed(c.range(), text)).collect();
        self.apply(edits, typing);
    }

    /// The blanks a line starts with.
    pub(super) fn leading(&self, line: usize) -> String {
        self.buffer.slice(self.buffer.line_start(line)..self.first_text(line)).into_owned()
    }

    /// Breaks the line at every cursor: the new line has the indent of the old, and a level
    /// more after what opens a block.
    pub fn newline(&mut self) {
        let edit = |c: &super::Cursor| {
            let start = self.buffer.line_start(self.buffer.line_of(c.start()));
            let before = self.buffer.slice(start..c.start());
            let blanks = before.bytes().take_while(|byte| matches!(byte, b' ' | b'\t')).count();
            let opens = before.trim_end_matches([' ', '\t']).ends_with(['{', '(', '[', ':']);
            let indent = if opens { self.indent_unit() } else { String::new() };
            Edit::typed(c.range(), format!("\n{}{indent}", &before[..blanks]))
        };
        let edits = self.cursors.iter().map(edit).collect();
        self.apply(edits, false);
    }

    /// Deletes what is selected, or the grapheme (the word) before each cursor.
    pub fn delete_back(&mut self, word: bool) {
        self.delete_towards(false, word);
    }

    pub fn delete_forward(&mut self, word: bool) {
        self.delete_towards(true, word);
    }

    fn delete_towards(&mut self, forward: bool, word: bool) {
        let range = |c: &super::Cursor| match (c.is_empty(), forward, word) {
            (false, ..) => c.range(),
            (true, true, true) => c.head..self.word_right(c.head),
            (true, true, false) => c.head..self.grapheme_right(c.head),
            (true, false, true) => self.word_left(c.head)..c.head,
            (true, false, false) => self.grapheme_left(c.head)..c.head,
        };
        // Two cursors in one word delete it once: a range starts where the one before ends.
        let mut end = 0;
        let apart = |range: std::ops::Range<usize>| {
            let range = range.start.max(end)..range.end.max(end);
            end = range.end;
            range
        };
        let ranges: Vec<_> =
            self.cursors.iter().map(range).map(apart).filter(|range| !range.is_empty()).collect();
        // Where every cursor deletes, each stays where its text was; where some have nothing
        // to delete, all of them are taken through the edits.
        let caret = (ranges.len() == self.cursors.len()).then_some(0);
        let typing = caret.is_some();
        let edits = ranges.into_iter().map(|range| Edit { range, text: String::new(), caret }).collect();
        self.apply(edits, typing);
    }

    /// Indents the lines of a selection over more than one line; puts an indent at the
    /// cursors otherwise: to the next stop, with spaces.
    pub fn tab(&mut self) {
        let lines = |c: &super::Cursor| self.buffer.line_of(c.start()) != self.buffer.line_of(c.end());
        if self.cursors.iter().any(lines) {
            return self.indent_lines(true);
        }
        let tab = self.tab_size.max(1);
        let edit = |c: &super::Cursor| {
            let spaces = " ".repeat(tab - self.col_of(c.start()) % tab);
            Edit::typed(c.range(), if self.use_tabs { "\t".to_string() } else { spaces })
        };
        let edits = self.cursors.iter().map(edit).collect();
        self.apply(edits, false);
    }

    /// What copy takes: the selections, a line each; or, when nothing is selected, the
    /// lines of the cursors.
    pub fn copy_text(&self) -> String {
        if self.cursors.iter().all(|c| c.is_empty()) {
            let line = |c: &super::Cursor| format!("{}\n", self.buffer.line(self.buffer.line_of(c.head)));
            return self.cursors.iter().map(line).collect();
        }
        self.selected_texts().join("\n")
    }

    /// Copies, then takes out the selections, or the lines of the cursors.
    pub fn cut(&mut self) -> String {
        let text = self.copy_text();
        if self.cursors.iter().all(|c| c.is_empty()) {
            self.delete_lines();
        } else {
            self.insert_text("", false);
        }
        text
    }

    /// Puts `text` at every cursor; a line at each cursor when it has as many lines as
    /// there are cursors.
    pub fn paste(&mut self, text: &str) {
        let text = text.replace("\r\n", "\n");
        let lines: Vec<&str> = text.trim_end_matches('\n').split('\n').collect();
        if self.cursors.len() > 1 && lines.len() == self.cursors.len() {
            let edits =
                self.cursors.iter().zip(lines).map(|(c, line)| Edit::typed(c.range(), line)).collect();
            return self.apply(edits, false);
        }
        self.insert_text(&text, false);
    }
}
