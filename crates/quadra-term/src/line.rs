//! The command line at the shell's prompt (specs/18, "The command line"): what is typed from
//! where the input starts to the cursor, read from the shell's OSC 133 marks or, for a shell
//! that marks nothing, after a prompt's sign. What any completion needs, with no knowledge of
//! commands.

use std::cell::Cell;

use oneterm_vt::{CellContent, RowId, Semantic, Terminal};
use quadra_prim::Rect;
use quadra_reactive::{Signal, signal};
use quadra_tree::{NodeId, Tree, Widget};

use crate::session::Session;
use crate::widget::TerminalWidget;

/// The most rows that wrap into the cursor's that a line is read from.
const MOST_ROWS: u64 = 64;

/// The line being typed at the cursor.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandLine {
    /// What is typed, from where the input starts to the cursor.
    pub text: String,
    /// The cursor is where the shell takes a command: in the input the shell marked (OSC 133
    /// `B`, no `C` since), or, for a shell that marks nothing, after a prompt's sign.
    pub at_prompt: bool,
    /// The alternate screen is up; the text is then empty.
    pub alt_screen: bool,
    /// The cursor's cell in the box of the view, its width a cell's: while the view has the
    /// focus and shows the cursor.
    pub caret: Option<Rect>,
}

/// The session's line, followed once something asked for it.
pub(crate) struct Follow {
    line: Signal<CommandLine>,
    on: Cell<bool>,
}

impl Follow {
    pub(crate) fn new() -> Follow {
        Follow { line: signal(CommandLine::default()), on: Cell::new(false) }
    }
}

impl Session {
    /// The command line at the cursor, a signal kept from the first call on: read again after
    /// each burst of output, a focus change and a resize, and set when it differs.
    pub fn command_line(&self) -> Signal<CommandLine> {
        let follow = &self.0.line;
        if !follow.on.replace(true) {
            let (id, handle) = (self.0.view.get(), self.0.handle.borrow().clone());
            if let (Some(id), Some(handle)) = (id, handle) {
                handle.with(move |tree| follow_line(tree, id));
            }
        }
        follow.line
    }
}

/// Reads the line of the session a terminal's widget shows, when it is followed.
pub(crate) fn follow_line(tree: &mut Tree, id: NodeId) {
    let Some(w) = tree.widget_mut::<TerminalWidget>(id) else { return };
    let s = w.session.clone();
    // A widget whose view was disposed (its tab closed) may still be told of a focus change.
    if !s.0.line.on.get() || !s.0.line.line.is_alive() {
        return;
    }
    let caret = w.caret();
    let line = {
        let engine = s.engine();
        let alt_screen = engine.term.grid().alt_active();
        let (text, at_prompt) =
            if alt_screen { (String::new(), false) } else { read(&engine.term, engine.marked) };
        CommandLine { text, at_prompt, alt_screen, caret }
    };
    // Set with no lock held: what follows it may write to the program.
    if s.0.line.line.with_untracked(|now| *now != line) {
        s.0.line.line.set(line);
    }
}

/// The text typed before the cursor and whether it is at a prompt. `marked`: the shell has
/// sent OSC 133 `B`.
fn read(term: &Terminal, marked: bool) -> (String, bool) {
    let screen = term.screen();
    let cursor = screen.cursor();
    let (row, end) = (cursor.pos.row, cursor.pos.col + u16::from(cursor.pending_wrap));
    let mut first = row;
    while marked && row.0 - first.0 < MOST_ROWS && first.0 > screen.oldest().0 {
        match screen.row(RowId(first.0 - 1)).wrapped() {
            true => first = RowId(first.0 - 1),
            false => break,
        }
    }
    // The cells from the first row to the cursor, each with its text and zone.
    let mut cells: Vec<(Semantic, char, Option<&str>)> = Vec::new();
    for id in first.0..=row.0 {
        let line = screen.row(RowId(id));
        let last = if id == row.0 { end.min(screen.cols()) } else { screen.cols() };
        for col in 0..last {
            let cell = line.cell(col);
            if cell.width().is_spacer() {
                continue;
            }
            cells.push(match cell.content() {
                CellContent::Scalar(c) => (cell.semantic(), c, None),
                CellContent::Grapheme(g) => (cell.semantic(), ' ', Some(term.interner().resolve_grapheme(g))),
            });
        }
    }
    let text_of = |cells: &[(Semantic, char, Option<&str>)]| -> String {
        cells.iter().fold(String::new(), |mut out, (_, c, cluster)| {
            match cluster {
                Some(cluster) => out.push_str(cluster),
                None => out.push(*c),
            }
            out
        })
    };
    if marked {
        let start = cells.iter().rposition(|(zone, ..)| *zone == Semantic::Prompt).map_or(0, |at| at + 1);
        return (text_of(&cells[start..]), term.template().semantic() == Semantic::Input);
    }
    strip_prompt(&text_of(&cells))
}

/// A row without the prompt before its command: after the first `>`, or `$ # ❯ ➜ λ` that a
/// space or the end follows, and the spaces after it. Not found, the row from its first
/// non-blank.
pub(crate) fn strip_prompt(row: &str) -> (String, bool) {
    let chars: Vec<char> = row.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let sign = c == '>'
            || (matches!(c, '$' | '#' | '❯' | '➜' | 'λ') && chars.get(i + 1).is_none_or(|next| *next == ' '));
        if sign {
            let rest = chars[i + 1..].iter().skip_while(|c| **c == ' ');
            return (rest.collect(), true);
        }
    }
    (row.trim_start().to_string(), false)
}

#[cfg(test)]
#[test]
fn prompts_are_cut_by_their_sign() {
    assert_eq!(strip_prompt("C:\\Users\\me>di"), ("di".into(), true));
    assert_eq!(strip_prompt("PS C:\\Users\\me> Get-Ch"), ("Get-Ch".into(), true));
    assert_eq!(strip_prompt("me@box:~/src$ git c"), ("git c".into(), true));
    assert_eq!(strip_prompt("me@box:~/src$"), ("".into(), true));
    assert_eq!(strip_prompt("  some text"), ("some text".into(), false));
}
