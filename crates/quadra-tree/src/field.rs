//! The text field widget (specs/04, "Widgets"; behaviour/09): editable text, on one line
//! that scrolls sideways so the caret stays in view, or on as many lines as it takes at the
//! field's width. It owns the edit buffer and paints the selection, the text and the caret.
//! What edits it is the view around it (handlers and actions).

mod widget;

use std::borrow::Cow;
use std::ops::Range;

use quadra_prim::{Color, Point, Rect};
use quadra_text::{EditBuffer, Motion, Paragraph, TextStyle, TextSystem, next_grapheme};

use crate::action::Action;
use crate::fonts::GlyphTile;

/// What a key binding asks of a text field. One action type with the command as its data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextCommand {
    /// Moves the caret; with `true` the selection is extended.
    Move(Motion, bool),
    /// Deletes the selection, or from the caret to where the motion leads.
    Delete(Motion),
    SelectAll,
    Undo,
    Redo,
    /// The clipboard is the view's to reach: the field only removes what a cut took.
    Copy,
    Cut,
    Paste,
    /// Starts a new line: bound where a field has several.
    Newline,
}

impl Action for TextCommand {
    const NAME: &'static str = "text::edit";
}

/// Motions that go by the lines a field shows.
fn by_line(motion: Motion) -> bool {
    matches!(motion, Motion::Up | Motion::Down | Motion::LineStart | Motion::LineEnd)
}

/// What a masked field shows for each character of its text.
const DOT: &str = "\u{2022}";

/// How many characters, as a reader counts them, `text` has.
fn characters(text: &str) -> usize {
    std::iter::successors(Some(0), |&at| (at < text.len()).then(|| next_grapheme(text, at))).count() - 1
}

pub struct TextField {
    pub buffer: EditBuffer,
    /// A password: a dot shows for each character, and nothing of it is copied.
    pub masked: bool,
    /// The text is on as many lines as it takes at the field's width, and can hold line
    /// breaks. Otherwise it is one line that scrolls sideways.
    pub multiline: bool,
    /// The width the lines were last broken at.
    width: f32,
    /// Where the caret wants to be while it moves up and down: it keeps its x through lines
    /// that are too short for it.
    goal_x: Option<f32>,
    style: TextStyle,
    pub color: Color,
    /// Behind the selected text.
    pub selection: Color,
    /// It has the focus: the caret shows.
    pub focused: bool,
    /// The caret's blink: shown now, and touched since the blink last turned (then it
    /// stays shown for a whole turn more).
    pub caret_on: bool,
    pub touched: bool,
    /// The text an input method is composing, and the caret in it: shown at the caret, in
    /// place of the selection, underlined, and not part of the buffer until it is committed.
    pub preedit: Option<(String, usize)>,
    /// Where the caret was painted, from the field's corner: where an input method opens.
    caret_at: Option<Rect>,
    paragraph: Option<Paragraph>,
    /// How far the text is moved left so that the caret is in view.
    scroll_x: f32,
    tiles: Vec<GlyphTile>,
}

impl TextField {
    pub fn new(text: &str, style: TextStyle, color: Color) -> TextField {
        TextField {
            buffer: EditBuffer::new(text),
            masked: false,
            multiline: false,
            width: f32::INFINITY,
            goal_x: None,
            style,
            color,
            selection: Color::rgb(0x3366FF).alpha(0.3),
            focused: false,
            caret_on: true,
            touched: false,
            preedit: None,
            caret_at: None,
            paragraph: None,
            scroll_x: 0.0,
            tiles: Vec::new(),
        }
    }

    /// Runs a command on the buffer. Returns whether the text changed. The paragraph must be
    /// that of the buffer's text ([`TextField::sync`]): the lines are read from it.
    pub fn run(&mut self, command: TextCommand) -> bool {
        let goal = self.goal_x.take();
        match command {
            TextCommand::Move(motion, extend) if self.multiline && by_line(motion) => {
                let Some(lines) = &self.paragraph else { return false };
                let (line, x) = lines.place(self.buffer.head());
                let (x, last) = (goal.unwrap_or(x), lines.lines().len().saturating_sub(1));
                let to = match motion {
                    Motion::Up if line == 0 => 0, // from the first line: to its start
                    Motion::Up => lines.index_near(line - 1, x),
                    Motion::Down if line == last => self.buffer.text().len(),
                    Motion::Down => lines.index_near(line + 1, x),
                    Motion::LineStart => lines.lines()[line].start,
                    _ => lines.line_end(line),
                };
                self.buffer.set_caret(to, extend);
                self.goal_x = matches!(motion, Motion::Up | Motion::Down).then_some(x);
            }
            TextCommand::Move(motion, extend) => self.buffer.move_to(motion, extend),
            TextCommand::Newline => return self.multiline && self.buffer.insert("\n"),
            TextCommand::Delete(motion) => return self.buffer.delete(motion),
            TextCommand::SelectAll => self.buffer.select_all(),
            TextCommand::Undo => return self.buffer.undo(),
            TextCommand::Redo => return self.buffer.redo(),
            TextCommand::Cut => return self.buffer.delete(Motion::Right),
            TextCommand::Copy | TextCommand::Paste => {}
        }
        false
    }

    /// Turns the caret's blink: hidden, then shown. A caret that was just used stays shown.
    pub fn blink(&mut self) {
        self.caret_on = std::mem::take(&mut self.touched) || !self.caret_on;
    }

    /// What is shown: the buffer's text, with the text being composed in place of the
    /// selection, or a dot for each character of that. Also where the caret is in it, the
    /// range that is being composed and the range that is selected.
    fn shown(&self) -> (Cow<'_, str>, usize, Range<usize>, Range<usize>) {
        let (text, selected) = (self.buffer.text(), self.buffer.selection());
        let (plain, caret, composed) = match &self.preedit {
            None => (Cow::Borrowed(text), self.buffer.head(), 0..0),
            Some((composing, caret)) => {
                let shown = [&text[..selected.start], composing, &text[selected.end..]].concat();
                let composed = selected.start..selected.start + composing.len();
                (Cow::Owned(shown), selected.start + (*caret).min(composing.len()), composed)
            }
        };
        if !self.masked {
            return (plain, caret, composed, selected);
        }
        // An offset in the dots: as many dots as there are characters before it.
        let at = |index: usize| characters(&plain[..index.min(plain.len())]) * DOT.len();
        let dots = DOT.repeat(characters(&plain));
        (
            Cow::Owned(dots),
            at(caret),
            at(composed.start)..at(composed.end),
            at(selected.start)..at(selected.end),
        )
    }

    /// The byte offset nearest a point from the field's corner: where a click puts the caret.
    pub fn index_at(&self, at: Point) -> usize {
        let Some(lines) = &self.paragraph else { return 0 };
        let line = (at.y / self.style.line_px()).floor().max(0.0) as usize;
        let shown = lines.index_near(line.min(lines.lines().len().saturating_sub(1)), at.x + self.scroll_x);
        if !self.masked {
            return shown;
        }
        let text = self.buffer.text();
        (0..shown / DOT.len()).fold(0, |at, _| next_grapheme(text, at))
    }

    /// Makes the paragraph that of the text as it is now, broken at the width the field was
    /// last laid out at. A layout does it; what reads the lines before the next one (a key
    /// that moves the caret up, a click) does it first.
    pub fn sync(&mut self, text: &mut TextSystem) {
        let shown = self.shown().0.into_owned();
        let paragraph = match &mut self.paragraph {
            Some(paragraph) => {
                paragraph.set_text(text, &shown);
                paragraph
            }
            none => none.insert(Paragraph::new(text, shown, self.style.clone())),
        };
        paragraph.wrap(text, if self.multiline { self.width } else { f32::INFINITY });
    }

    /// Where the caret is, from the field's corner, as the text is laid out now.
    pub fn caret_rect(&self) -> Option<Rect> {
        let (lines, line_px) = (self.paragraph.as_ref()?, self.style.line_px());
        let (line, x) = lines.place(self.shown().1);
        Some(Rect::new(x - self.scroll_x, line as f32 * line_px, 1.0, line_px))
    }
}
