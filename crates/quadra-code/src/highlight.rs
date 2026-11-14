//! The highlighter (behaviour/18, "Highlighting"): the state each line ends in, read again
//! from the first line a change touched, and only as far as a view asks. A letter typed
//! deep in a large file costs the lines that show, not the file.

use crate::buffer::Buffer;
use crate::doc::Doc;
use crate::token::{Token, Tokenizer};

/// What colours the lines of a document for the editor. The highlighter of line states is
/// one; one that parses the whole text and keeps a tree of it (fed the changes since the
/// last look) can be another, giving the tokens of a line from its tree.
pub trait Highlight {
    /// Takes what changed in the document since the last look.
    fn sync(&mut self, doc: &mut Doc);
    /// Puts the tokens of `line` in `out` (cleared first), in order, none over another.
    fn tokens(&mut self, buffer: &Buffer, line: usize, out: &mut Vec<Token>);
}

impl Highlight for Highlighter {
    fn sync(&mut self, doc: &mut Doc) {
        Highlighter::sync(self, doc);
    }

    fn tokens(&mut self, buffer: &Buffer, line: usize, out: &mut Vec<Token>) {
        Highlighter::tokens(self, buffer, line, out);
    }
}

pub struct Highlighter {
    pub tokenizer: Tokenizer,
    /// The state each line ends in, for the lines before `valid`.
    ends: Vec<u32>,
    /// Lines from here on are stale.
    valid: usize,
}

impl Highlighter {
    pub fn new(tokenizer: Tokenizer) -> Highlighter {
        Highlighter { tokenizer, ends: Vec::new(), valid: 0 }
    }

    /// Takes the first line the document changed since the last look.
    pub fn sync(&mut self, doc: &mut Doc) {
        self.invalidate(doc.dirty);
        doc.dirty = usize::MAX;
    }

    /// Lines from `line` on are read again when asked for.
    pub fn invalidate(&mut self, line: usize) {
        self.valid = self.valid.min(line);
        self.ends.truncate(self.valid);
    }

    /// The lines whose end states are known.
    pub fn valid(&self) -> usize {
        self.valid
    }

    /// The state `line` starts in, reading the stale lines before it.
    pub fn start_state(&mut self, buffer: &Buffer, line: usize) -> u32 {
        let line = line.min(buffer.line_count());
        if self.tokenizer == Tokenizer::Plain {
            return 0; // nothing is open at the end of a line
        }
        let mut out = Vec::new();
        while self.valid < line {
            let state = self.ends.last().copied().unwrap_or(0);
            out.clear();
            let end = self.tokenizer.tokenize(&buffer.line(self.valid), state, &mut out);
            self.ends.push(end);
            self.valid += 1;
        }
        if line == 0 { 0 } else { self.ends[line - 1] }
    }

    /// Puts the tokens of `line` in `out` (cleared first).
    pub fn tokens(&mut self, buffer: &Buffer, line: usize, out: &mut Vec<Token>) {
        let state = self.start_state(buffer, line);
        out.clear();
        let end = self.tokenizer.tokenize(&buffer.line(line), state, out);
        if self.valid == line && line < buffer.line_count() {
            self.ends.push(end);
            self.valid += 1;
        }
    }
}
