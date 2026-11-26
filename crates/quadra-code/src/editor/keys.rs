//! The editor's keys (behaviour/18, "Keys"): its commands as an action, bound in the
//! context `editor`.

use quadra_tree::{Action, Keymap};

use crate::doc::{Doc, Move as Motion};

/// The key context of the editor.
pub const EDITOR_CONTEXT: &str = "editor";

/// A command of the editor: what its keys do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorCommand {
    /// Moves the cursors; `true` extends their selections.
    Move(Motion, bool),
    /// Deletes before the cursors, or after; `true`: a word.
    DeleteBack(bool),
    DeleteForward(bool),
    Newline,
    Tab,
    Outdent,
    Undo,
    Redo,
    SelectAll,
    SelectLine,
    SelectNext,
    /// A cursor more below, or above.
    AddCursor(bool),
    DeleteLines,
    DuplicateLines,
    /// Moves the lines of the cursors down, or up.
    MoveLines(bool),
    ToggleComment,
    Copy,
    Cut,
    Paste,
    /// One cursor, where the primary is; with the find bar open, the bar closes instead.
    Collapse,
    /// Opens the find bar, or with the replacing row.
    Find,
    Replace,
    /// Selects the next match, or with `true` the one before.
    FindNext(bool),
}

// ponytail: one action with data, so a keymap file cannot name a command yet; names for
// them (`Actions::add_with`) when a user's file needs them.
impl Action for EditorCommand {
    const NAME: &'static str = "editor::Command";
}

/// Adds the editor's keys to a keymap.
pub fn bind_keys(keymap: &mut Keymap) {
    use EditorCommand::*;
    let well_formed = "the editor's bindings are well formed";
    let moves = [
        ("left", Motion::Left),
        ("right", Motion::Right),
        ("up", Motion::Up),
        ("down", Motion::Down),
        ("ctrl-left", Motion::WordLeft),
        ("ctrl-right", Motion::WordRight),
        ("home", Motion::Home),
        ("end", Motion::End),
        ("ctrl-home", Motion::DocStart),
        ("ctrl-end", Motion::DocEnd),
        ("pageup", Motion::PageUp),
        ("pagedown", Motion::PageDown),
    ];
    for (keys, motion) in moves {
        keymap.bind_in(EDITOR_CONTEXT, keys, Move(motion, false)).expect(well_formed);
        keymap.bind_in(EDITOR_CONTEXT, &format!("shift-{keys}"), Move(motion, true)).expect(well_formed);
    }
    let rest = [
        ("backspace", DeleteBack(false)),
        ("shift-backspace", DeleteBack(false)),
        ("ctrl-backspace", DeleteBack(true)),
        ("delete", DeleteForward(false)),
        ("ctrl-delete", DeleteForward(true)),
        ("enter", Newline),
        ("tab", Tab),
        ("shift-tab", Outdent),
        ("ctrl-z", Undo),
        ("ctrl-y", Redo),
        ("ctrl-shift-z", Redo),
        ("ctrl-a", SelectAll),
        ("ctrl-l", SelectLine),
        ("ctrl-d", SelectNext),
        ("ctrl-alt-down", AddCursor(true)),
        ("ctrl-alt-up", AddCursor(false)),
        ("alt-shift-down", AddCursor(true)),
        ("alt-shift-up", AddCursor(false)),
        ("alt-shift-left", Move(Motion::Left, true)),
        ("alt-shift-right", Move(Motion::Right, true)),
        ("ctrl-shift-k", DeleteLines),
        ("ctrl-shift-d", DuplicateLines),
        ("alt-down", MoveLines(true)),
        ("alt-up", MoveLines(false)),
        ("ctrl-/", ToggleComment),
        ("ctrl-c", Copy),
        ("ctrl-insert", Copy),
        ("ctrl-x", Cut),
        ("shift-delete", Cut),
        ("ctrl-v", Paste),
        ("shift-insert", Paste),
        ("escape", Collapse),
        ("ctrl-f", Find),
        ("ctrl-h", Replace),
        ("f3", FindNext(false)),
        ("shift-f3", FindNext(true)),
    ];
    for (keys, command) in rest {
        keymap.bind_in(EDITOR_CONTEXT, keys, command).expect(well_formed);
    }
    // On a Mac (specs/09, "On a Mac"): by words with Option, to the ends with Command, and
    // the keys its menu bar keeps (Command-H hides the app) are left to it.
    let moves = [
        ("alt-left", Motion::WordLeft),
        ("alt-right", Motion::WordRight),
        ("ctrl-left", Motion::Home),
        ("ctrl-right", Motion::End),
        ("ctrl-up", Motion::DocStart),
        ("ctrl-down", Motion::DocEnd),
    ];
    for (keys, motion) in moves {
        keymap.bind_in_on_mac(EDITOR_CONTEXT, keys, Move(motion, false)).expect(well_formed);
        let keys = format!("shift-{keys}");
        keymap.bind_in_on_mac(EDITOR_CONTEXT, &keys, Move(motion, true)).expect(well_formed);
    }
    // ponytail: Command-Backspace takes a word as Option-Backspace does, not the line before
    // the caret: the document has no such delete yet.
    let rest = [
        ("alt-backspace", DeleteBack(true)),
        ("alt-delete", DeleteForward(true)),
        ("ctrl-alt-f", Replace),
        ("ctrl-g", FindNext(false)),
        ("ctrl-shift-g", FindNext(true)),
    ];
    for (keys, command) in rest {
        keymap.bind_in_on_mac(EDITOR_CONTEXT, keys, command).expect(well_formed);
    }
}

impl EditorCommand {
    /// Does the command to a document; `page` is how many lines a page is. The clipboard's
    /// commands and finding are left to the view, which has the clipboard and the bar.
    pub fn run(self, doc: &mut Doc, page: usize) {
        use EditorCommand::*;
        match self {
            Move(motion, extend) => {
                doc.move_cursors(motion, extend, page);
                doc.break_group();
            }
            DeleteBack(word) => doc.delete_back(word),
            DeleteForward(word) => doc.delete_forward(word),
            Newline => doc.newline(),
            Tab => doc.tab(),
            Outdent => doc.indent_lines(false),
            Undo => _ = doc.undo(),
            Redo => _ = doc.redo(),
            SelectAll => doc.select_all(),
            SelectLine => doc.select_line(),
            SelectNext => doc.select_next(),
            AddCursor(down) => doc.add_cursor_vertical(down),
            DeleteLines => doc.delete_lines(),
            DuplicateLines => doc.duplicate_lines(),
            MoveLines(down) => doc.move_lines(down),
            ToggleComment => doc.toggle_comment(),
            Collapse => doc.cursors = vec![doc.primary()],
            Copy | Cut | Paste | Find | Replace | FindNext(_) => {}
        }
    }
}
