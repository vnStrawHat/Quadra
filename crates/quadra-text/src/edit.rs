//! The text an input edits (specs/08, "Editing support"; behaviour/09, "Text input
//! internals"): the text, a selection as an anchor and a head, movement by graphemes and
//! words, and an undo history in which typing is one step per word.

use std::ops::Range;

use crate::grapheme::{next_grapheme, prev_grapheme};
use crate::normalize::{is_mark, normalize_nfc};

/// Where the head goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    Left,
    Right,
    WordLeft,
    WordRight,
    Start,
    End,
    /// To the start or the end of the line: here between line breaks; a field that wraps
    /// its text goes by the lines it shows.
    LineStart,
    LineEnd,
    /// A line up or down. Only what lays the text out knows where that is: the buffer
    /// stays where it is.
    Up,
    Down,
}

/// One change, as undo needs it: what was at `at` before and after, and the selections.
struct Change {
    at: usize,
    removed: String,
    inserted: String,
    before: (usize, usize),
    after: (usize, usize),
    /// Typing that the next typed character may join.
    typing: bool,
}

#[derive(Default)]
pub struct EditBuffer {
    text: String,
    /// The fixed end of the selection, and the end that moves: the caret.
    anchor: usize,
    head: usize,
    undo: Vec<Change>,
    redo: Vec<Change>,
}

#[derive(PartialEq)]
enum Class {
    Space,
    Word,
    Other,
}

fn class(c: char) -> Class {
    if c.is_whitespace() {
        Class::Space
    } else if c.is_alphanumeric() || c == '_' {
        Class::Word
    } else {
        Class::Other
    }
}

impl EditBuffer {
    pub fn new(text: impl Into<String>) -> EditBuffer {
        let text = text.into();
        let end = text.len();
        EditBuffer { text, anchor: end, head: end, ..EditBuffer::default() }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// The caret: the end of the selection that moves.
    pub fn head(&self) -> usize {
        self.head
    }

    /// What is selected; empty when there is only a caret.
    pub fn selection(&self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }

    pub fn selected_text(&self) -> &str {
        &self.text[self.selection()]
    }

    /// Replaces the text from outside (the value it shows changed). The selection is kept
    /// where it still fits; the history is of another text and is dropped.
    pub fn set_text(&mut self, text: &str) {
        if self.text != text {
            self.text = text.to_string();
            let fit = |i: usize| if self.text.is_char_boundary(i) { i } else { self.text.len() };
            (self.anchor, self.head) = (fit(self.anchor.min(text.len())), fit(self.head.min(text.len())));
            (self.undo, self.redo) = (Vec::new(), Vec::new());
        }
    }

    /// The start of the word at or before `i`, and the end of the word at or after it: words
    /// are runs of letters and digits, of spaces, or of anything else.
    fn word_left(&self, i: usize) -> usize {
        let mut chars = self.text[..i].char_indices().rev().peekable();
        while chars.next_if(|&(_, c)| class(c) == Class::Space).is_some() {}
        let Some(&(mut start, first)) = chars.peek() else { return 0 };
        let kind = class(first);
        while let Some((at, _)) = chars.next_if(|&(_, c)| class(c) == kind) {
            start = at;
        }
        start
    }

    fn word_right(&self, i: usize) -> usize {
        let mut chars = self.text[i..].char_indices().peekable();
        while chars.next_if(|&(_, c)| class(c) == Class::Space).is_some() {}
        let Some(&(_, first)) = chars.peek() else { return self.text.len() };
        let kind = class(first);
        while chars.next_if(|&(_, c)| class(c) == kind).is_some() {}
        chars.peek().map_or(self.text.len(), |&(at, _)| i + at)
    }

    fn target(&self, motion: Motion) -> usize {
        match motion {
            Motion::Left => prev_grapheme(&self.text, self.head),
            Motion::Right => next_grapheme(&self.text, self.head),
            Motion::WordLeft => self.word_left(self.head),
            Motion::WordRight => self.word_right(self.head),
            Motion::Start => 0,
            Motion::End => self.text.len(),
            Motion::LineStart => self.text[..self.head].rfind('\n').map_or(0, |at| at + 1),
            Motion::LineEnd => self.text[self.head..].find('\n').map_or(self.text.len(), |at| self.head + at),
            Motion::Up | Motion::Down => self.head,
        }
    }

    /// Moves the caret; with `extend` the anchor stays and the selection grows or shrinks. A
    /// step left or right out of a selection lands on that end of it.
    pub fn move_to(&mut self, motion: Motion, extend: bool) {
        let selection = self.selection();
        self.head = match motion {
            Motion::Left if !extend && !selection.is_empty() => selection.start,
            Motion::Right if !extend && !selection.is_empty() => selection.end,
            _ => self.target(motion),
        };
        if !extend {
            self.anchor = self.head;
        }
        self.settle();
    }

    /// Puts the caret at a byte offset (a click), or drags the selection to it.
    pub fn set_caret(&mut self, index: usize, extend: bool) {
        let mut index = index.min(self.text.len());
        while !self.text.is_char_boundary(index) {
            index -= 1;
        }
        self.head = index;
        if !extend {
            self.anchor = index;
        }
        self.settle();
    }

    pub fn select_all(&mut self) {
        (self.anchor, self.head) = (0, self.text.len());
        self.settle();
    }

    /// Selects the word around a byte offset: a double click.
    pub fn select_word(&mut self, index: usize) {
        self.set_caret(index, false);
        let end = self.word_right(self.head);
        (self.anchor, self.head) = (self.word_left(end), end);
        self.settle();
    }

    /// A movement ends a run of typing: what is typed next is a new undo step.
    fn settle(&mut self) {
        if let Some(last) = self.undo.last_mut() {
            last.typing = false;
        }
    }

    /// Replaces a range, and records it. Returns whether the text changed.
    fn replace(&mut self, range: Range<usize>, with: &str, typing: bool) -> bool {
        if range.is_empty() && with.is_empty() {
            return false;
        }
        let before = (self.anchor, self.head);
        let removed = self.text[range.clone()].to_string();
        self.text.replace_range(range.clone(), with);
        (self.anchor, self.head) = (range.start + with.len(), range.start + with.len());
        let after = (self.anchor, self.head);
        self.redo.clear();
        // Typing joins the step before it while it stays in one word.
        let joins = typing && with.chars().all(|c| class(c) != Class::Space);
        match self.undo.last_mut().filter(|last| joins && last.typing && last.after == before) {
            Some(last) => {
                last.inserted.push_str(with);
                last.after = after;
            }
            None => self.undo.push(Change {
                at: range.start,
                removed,
                inserted: with.to_string(),
                before,
                after,
                // A space is a step of its own, and so is typing over a selection.
                typing: joins && range.is_empty(),
            }),
        }
        true
    }

    /// Types text over the selection.
    pub fn insert(&mut self, text: &str) -> bool {
        let (mut range, mut with) = (self.selection(), normalize_nfc(text));
        // A combining mark typed after a letter composes with it: the letter, and the marks
        // it has already, are replaced by what they make together.
        if with.chars().next().is_some_and(is_mark) {
            let before = self.text[..range.start].char_indices().rev();
            let letter = { before }.find(|&(_, c)| !is_mark(c)).map_or(0, |(at, _)| at);
            let joined = format!("{}{with}", &self.text[letter..range.start]);
            with = normalize_nfc(&joined).into_owned().into();
            range.start = letter;
        }
        self.replace(range, &with, true)
    }

    /// Deletes the selection, or from the caret to where the motion leads: Backspace is
    /// `Left`, Delete is `Right`.
    pub fn delete(&mut self, motion: Motion) -> bool {
        let selection = self.selection();
        let range = if selection.is_empty() {
            let to = self.target(motion);
            to.min(self.head)..to.max(self.head)
        } else {
            selection
        };
        self.replace(range, "", false)
    }

    /// Takes back the last step; the selection is as it was before it.
    pub fn undo(&mut self) -> bool {
        let Some(change) = self.undo.pop() else { return false };
        self.text.replace_range(change.at..change.at + change.inserted.len(), &change.removed);
        (self.anchor, self.head) = change.before;
        self.redo.push(change);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(change) = self.redo.pop() else { return false };
        self.text.replace_range(change.at..change.at + change.removed.len(), &change.inserted);
        (self.anchor, self.head) = change.after;
        self.undo.push(Change { typing: false, ..change });
        true
    }
}
