//! A document (specs/13, "Code"; behaviour/18): a buffer with its cursors, what was done to
//! it in groups that undo takes back, and the commands of editing. Nothing here draws or
//! knows of keys: the editor calls it, and so does a test.

mod commands;
mod lines;
mod motion;
mod select;

use std::ops::Range;

use crate::buffer::Buffer;

pub use motion::Move;

/// The groups undo keeps: the oldest goes when there is one more.
// ponytail: counted in groups, not in bytes; a budget of bytes when a session pastes
// megabytes a thousand times.
const MOST_UNDO: usize = 1000;
/// Letters typed within this of each other, in milliseconds, are undone together.
const TYPING_PAUSE: u64 = 1000;

/// A cursor: its selection is from `anchor` to `head`, and it is where `head` is.
#[derive(Clone, Copy, Debug, Default)]
pub struct Cursor {
    pub anchor: usize,
    pub head: usize,
    /// The column it goes back to on a longer line, kept while it moves up and down.
    pub goal: Option<usize>,
}

/// Cursors are the same when their selections are: where one would go is not part of it.
impl PartialEq for Cursor {
    fn eq(&self, other: &Cursor) -> bool {
        (self.anchor, self.head) == (other.anchor, other.head)
    }
}

impl Cursor {
    pub fn at(offset: usize) -> Cursor {
        Cursor { anchor: offset, head: offset, goal: None }
    }

    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }

    pub fn range(&self) -> Range<usize> {
        self.start()..self.end()
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }
}

/// `text` in place of `range`. With a `caret`, the cursor that asked for the edit lands
/// that far into the text; without one in any edit, every cursor stays with its text.
#[derive(Clone, Debug, Default)]
pub struct Edit {
    pub range: Range<usize>,
    pub text: String,
    pub caret: Option<usize>,
}

impl Edit {
    /// An edit that leaves its cursor after its text.
    pub fn typed(range: Range<usize>, text: impl Into<String>) -> Edit {
        let text = text.into();
        Edit { range, caret: Some(text.len()), text }
    }
}

/// What an edit did, for undo to take back.
struct Change {
    at: usize,
    removed: String,
    inserted: String,
}

/// What undo takes back at once, and the cursors before and after it.
struct Group {
    changes: Vec<Change>,
    before: Vec<Cursor>,
    after: Vec<Cursor>,
    /// Typing that the next letter may join.
    typing: bool,
    at: u64,
}

pub struct Doc {
    pub buffer: Buffer,
    /// One at least, in the order of the text, none over another.
    pub cursors: Vec<Cursor>,
    pub tab_size: usize,
    /// A level of indent is a tab, not `tab_size` spaces.
    pub use_tabs: bool,
    /// What starts a comment to the end of the line.
    pub comment: String,
    /// Counts the changes of the text: what a view keeps of it is as old as this was.
    pub version: u64,
    /// The first line changed since who colours the text last looked; `usize::MAX`: none.
    pub dirty: usize,
    /// The time of the command that comes, in milliseconds: whoever drives the document
    /// sets it, and typing after a pause is a group of its own.
    pub now: u64,
    undo: Vec<Group>,
    redo: Vec<Group>,
}

impl Doc {
    pub fn new(text: &str) -> Doc {
        Doc {
            buffer: Buffer::new(text),
            cursors: vec![Cursor::default()],
            tab_size: 4,
            use_tabs: false,
            comment: "//".to_string(),
            version: 0,
            dirty: usize::MAX,
            now: 0,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// The cursor commands of one cursor work on: the last.
    pub fn primary(&self) -> Cursor {
        self.cursors[self.cursors.len() - 1]
    }

    pub fn text(&self) -> String {
        self.buffer.text()
    }

    pub fn selected_texts(&self) -> Vec<String> {
        self.cursors.iter().map(|cursor| self.buffer.slice(cursor.range()).into_owned()).collect()
    }

    /// Puts the cursors in the order of the text, and makes one of those that overlap, or
    /// that touch when one of them selects nothing.
    pub fn normalize(&mut self) {
        self.cursors.sort_by_key(Cursor::start);
        let mut out: Vec<Cursor> = Vec::with_capacity(self.cursors.len());
        for cursor in &self.cursors {
            match out.last_mut() {
                Some(last)
                    if cursor.start() < last.end()
                        || (cursor.start() == last.end() && (cursor.is_empty() || last.is_empty())) =>
                {
                    let (start, end) = (last.start(), last.end().max(cursor.end()));
                    let forward = last.head >= last.anchor;
                    *last = if forward {
                        Cursor { anchor: start, head: end, goal: None }
                    } else {
                        Cursor { anchor: end, head: start, goal: None }
                    };
                }
                _ => out.push(*cursor),
            }
        }
        self.cursors = out;
    }

    fn touch(&mut self, offset: usize) {
        self.dirty = self.dirty.min(self.buffer.line_of(offset));
    }

    /// Makes the edits, which are in the order of the text and not over each other, as one
    /// change. `typing` joins it to typing just before it.
    pub fn apply(&mut self, edits: Vec<Edit>, typing: bool) {
        let Some(first) = edits.first() else { return };
        debug_assert!(
            edits.windows(2).all(|pair| pair[0].range.end <= pair[1].range.start),
            "edits over each other"
        );
        self.touch(first.range.start);
        let before = self.cursors.clone();
        let mut changes = Vec::with_capacity(edits.len());
        // The last first: the offsets of those before it stay what they were.
        for edit in edits.iter().rev() {
            let removed = self.buffer.slice(edit.range.clone()).into_owned();
            self.buffer.delete(edit.range.clone());
            self.buffer.insert(edit.range.start, &edit.text);
            changes.push(Change { at: edit.range.start, removed, inserted: edit.text.clone() });
        }
        // With carets an edit puts the cursor that asked for it; without, every cursor is
        // taken through the edits: an edit a line, not an edit a cursor.
        if edits.iter().any(|edit| edit.caret.is_some()) {
            let mut shift = 0isize;
            self.cursors.clear();
            for edit in &edits {
                let caret =
                    edit.range.start.wrapping_add_signed(shift) + edit.caret.unwrap_or(edit.text.len());
                self.cursors.push(Cursor::at(caret));
                shift += edit.text.len() as isize - edit.range.len() as isize;
            }
        } else {
            let through = |offset: usize| map_through(offset, &edits);
            self.cursors = before
                .iter()
                .map(|c| Cursor { anchor: through(c.anchor), head: through(c.head), goal: None })
                .collect();
        }
        self.normalize();
        self.version += 1;
        self.redo.clear();
        let lines = edits.iter().any(|edit| edit.text.contains('\n'));
        match self.undo.last_mut() {
            Some(last)
                if typing
                    && last.typing
                    && !lines
                    && self.now.saturating_sub(last.at) < TYPING_PAUSE
                    && last.after == before =>
            {
                last.changes.append(&mut changes);
                (last.after, last.at) = (self.cursors.clone(), self.now);
            }
            _ => {
                self.undo.push(Group { changes, before, after: self.cursors.clone(), typing, at: self.now });
                if self.undo.len() > MOST_UNDO {
                    self.undo.remove(0);
                }
            }
        }
    }

    /// Ends the group typing joins: the cursor moved, or something else was done.
    pub fn break_group(&mut self) {
        if let Some(last) = self.undo.last_mut() {
            last.typing = false;
        }
    }

    /// Takes the last group back. False when there is none.
    pub fn undo(&mut self) -> bool {
        let Some(group) = self.undo.pop() else { return false };
        for change in group.changes.iter().rev() {
            self.touch(change.at);
            self.buffer.delete(change.at..change.at + change.inserted.len());
            self.buffer.insert(change.at, &change.removed);
        }
        self.cursors.clone_from(&group.before);
        self.version += 1;
        self.redo.push(group);
        true
    }

    /// Does again the last group taken back. False when there is none.
    pub fn redo(&mut self) -> bool {
        let Some(mut group) = self.redo.pop() else { return false };
        for change in &group.changes {
            self.touch(change.at);
            self.buffer.delete(change.at..change.at + change.removed.len());
            self.buffer.insert(change.at, &change.inserted);
        }
        self.cursors.clone_from(&group.after);
        self.version += 1;
        group.typing = false;
        self.undo.push(group);
        true
    }
}

/// Where an offset is after edits in the order of the text: one where text is put in stays
/// before it, one in what was replaced lands after the new text.
fn map_through(offset: usize, edits: &[Edit]) -> usize {
    let mut shift = 0isize;
    for edit in edits {
        let Range { start, end } = edit.range;
        if offset < end || (offset == end && start == end) {
            let at = if offset <= start { offset } else { start + edit.text.len() };
            return at.wrapping_add_signed(shift);
        }
        shift += edit.text.len() as isize - edit.range.len() as isize;
    }
    offset.wrapping_add_signed(shift)
}
