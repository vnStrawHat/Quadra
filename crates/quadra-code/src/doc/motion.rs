//! Where cursors go: by grapheme, by word, by line, by column.

use quadra_text::{cell_step, next_grapheme, prev_grapheme};

use super::Doc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Left,
    Right,
    Up,
    Down,
    WordLeft,
    WordRight,
    /// To the first thing of the line that is not blank, and from there to its start.
    Home,
    End,
    DocStart,
    DocEnd,
    PageUp,
    PageDown,
}

/// What words are made of, what is between them, and the rest.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Class {
    Space,
    Word,
    Punct,
}

pub(super) fn class_of(byte: u8) -> Class {
    match byte {
        b' ' | b'\t' | b'\n' => Class::Space,
        _ if byte >= 0x80 || byte.is_ascii_alphanumeric() || byte == b'_' => Class::Word,
        _ => Class::Punct,
    }
}

/// How many bytes at the start of `bytes` (taken backwards when `back`) are blanks and
/// then things of one class: a word and the blanks before it.
fn word_run(bytes: &[u8], back: bool) -> usize {
    let at = |i: usize| if back { bytes[bytes.len() - 1 - i] } else { bytes[i] };
    let blanks = (0..bytes.len()).take_while(|i| matches!(at(*i), b' ' | b'\t')).count();
    let same = (blanks..bytes.len()).take_while(|i| class_of(at(*i)) == class_of(at(blanks))).count();
    blanks + same
}

/// The columns of a text: a tab goes to the next stop.
pub(crate) fn columns(text: &str, tab: usize) -> usize {
    let (mut col, mut at) = (0, 0);
    while at < text.len() {
        let (end, width) = cell_step(text, at);
        col = if text.as_bytes()[at] == b'\t' { (col / tab + 1) * tab } else { col + width };
        at = end;
    }
    col
}

impl Doc {
    /// The column an offset is in: tabs go to the next stop, and a wide character is two.
    pub fn col_of(&self, offset: usize) -> usize {
        let start = self.buffer.line_start(self.buffer.line_of(offset));
        columns(&self.buffer.slice(start..offset), self.tab_size.max(1))
    }

    /// The offset in a line nearest to a column.
    pub fn offset_at_col(&self, line: usize, col: usize) -> usize {
        let (text, tab) = (self.buffer.line(line), self.tab_size.max(1));
        let (mut at, mut c) = (0, 0);
        while at < text.len() {
            let (end, width) = cell_step(&text, at);
            let width = if text.as_bytes()[at] == b'\t' { (c / tab + 1) * tab - c } else { width };
            if c + width > col {
                // Past the middle of something wide, after it.
                at = if width > 1 && col - c > width / 2 { end } else { at };
                break;
            }
            (c, at) = (c + width, end);
        }
        self.buffer.line_start(line) + at
    }

    /// Where the word after an offset ends; at the end of a line, the start of the next.
    pub fn word_right(&self, offset: usize) -> usize {
        let line = self.buffer.line_of(offset);
        let (start, end) = (self.buffer.line_start(line), self.buffer.line_end(line));
        if offset >= end {
            return (offset + 1).min(self.buffer.len());
        }
        offset + word_run(&self.buffer.line(line).as_bytes()[offset - start..], false)
    }

    /// Where the word before an offset starts; at the start of a line, the end of the one
    /// before.
    pub fn word_left(&self, offset: usize) -> usize {
        let start = self.buffer.line_start(self.buffer.line_of(offset));
        if offset <= start {
            return offset.saturating_sub(1);
        }
        offset - word_run(self.buffer.slice(start..offset).as_bytes(), true)
    }

    pub(super) fn grapheme_left(&self, offset: usize) -> usize {
        let start = self.buffer.line_start(self.buffer.line_of(offset));
        if offset == start {
            return offset.saturating_sub(1);
        }
        start + prev_grapheme(&self.buffer.slice(start..offset), offset - start)
    }

    pub(super) fn grapheme_right(&self, offset: usize) -> usize {
        let line = self.buffer.line_of(offset);
        let (start, end) = (self.buffer.line_start(line), self.buffer.line_end(line));
        if offset >= end {
            return (offset + 1).min(self.buffer.len());
        }
        start + next_grapheme(&self.buffer.line(line), offset - start)
    }

    /// Where the first thing of a line that is not blank is.
    pub fn first_text(&self, line: usize) -> usize {
        let blanks = self.buffer.line(line).bytes().take_while(|byte| matches!(byte, b' ' | b'\t')).count();
        self.buffer.line_start(line) + blanks
    }

    /// Moves every cursor; `extend` leaves its anchor where it is. `page` is how many lines
    /// a page is.
    pub fn move_cursors(&mut self, motion: Move, extend: bool, page: usize) {
        let last = self.buffer.line_count() - 1;
        for i in 0..self.cursors.len() {
            let mut c = self.cursors[i];
            // A selection is left at the end the cursor goes to.
            if !extend && !c.is_empty() && matches!(motion, Move::Left | Move::Right) {
                let at = if motion == Move::Left { c.start() } else { c.end() };
                self.cursors[i] = super::Cursor::at(at);
                continue;
            }
            let vertical = matches!(motion, Move::Up | Move::Down | Move::PageUp | Move::PageDown);
            let goal = if vertical { c.goal.unwrap_or_else(|| self.col_of(c.head)) } else { 0 };
            let line = self.buffer.line_of(c.head);
            c.head = match motion {
                Move::Left => self.grapheme_left(c.head),
                Move::Right => self.grapheme_right(c.head),
                Move::WordLeft => self.word_left(c.head),
                Move::WordRight => self.word_right(c.head),
                Move::Home if c.head == self.first_text(line) => self.buffer.line_start(line),
                Move::Home => self.first_text(line),
                Move::End => self.buffer.line_end(line),
                Move::DocStart => 0,
                Move::DocEnd => self.buffer.len(),
                Move::Up if line == 0 => 0,
                Move::Up => self.offset_at_col(line - 1, goal),
                Move::Down if line == last => self.buffer.len(),
                Move::Down => self.offset_at_col(line + 1, goal),
                Move::PageUp => self.offset_at_col(line.saturating_sub(page), goal),
                Move::PageDown => self.offset_at_col((line + page).min(last), goal),
            };
            c.anchor = if extend { c.anchor } else { c.head };
            c.goal = vertical.then_some(goal);
            self.cursors[i] = c;
        }
        self.normalize();
        self.break_group();
    }
}
