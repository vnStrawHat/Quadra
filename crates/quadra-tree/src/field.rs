//! The text field widget (specs/04, "Widgets"; behaviour/09): one line of editable text. It
//! owns the edit buffer and paints the selection, the text and the caret, scrolled sideways
//! so the caret stays in view. What edits it is the view around it (handlers and actions).

use std::borrow::Cow;
use std::ops::Range;

use quadra_a11y::{Role, Semantics};
use quadra_layout::Known;
use quadra_prim::{Color, Point, Rect, Size};
use quadra_render::Quad;
use quadra_text::{EditBuffer, Motion, Paragraph, TextStyle, next_grapheme};

use crate::action::Action;
use crate::fonts::GlyphTile;
use crate::text::paint_glyphs;
use crate::widget::{MeasureCx, PaintCx, Widget};

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
}

impl Action for TextCommand {
    const NAME: &'static str = "text::edit";
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

    /// Runs a command on the buffer. Returns whether the text changed.
    pub fn run(&mut self, command: TextCommand) -> bool {
        match command {
            TextCommand::Move(motion, extend) => self.buffer.move_to(motion, extend),
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

    /// The byte offset nearest an x from the field's left edge: where a click puts the caret.
    pub fn index_at(&self, x: f32) -> usize {
        let shown = self.paragraph.as_ref().map_or(0, |p| p.index_at(x + self.scroll_x));
        if !self.masked {
            return shown;
        }
        let text = self.buffer.text();
        (0..shown / DOT.len()).fold(0, |at, _| next_grapheme(text, at))
    }
}

impl Widget for TextField {
    fn is_leaf(&self) -> bool {
        true
    }

    /// One line high, and as wide as the space it is given: the text scrolls inside it. Only
    /// where the space has no end is it as wide as its text.
    fn measure(&mut self, cx: &mut MeasureCx, known: Known, avail: Size) -> Size {
        let shown = self.shown().0.into_owned();
        let paragraph = match &mut self.paragraph {
            Some(paragraph) => {
                paragraph.set_text(cx.text, &shown);
                paragraph
            }
            none => none.insert(Paragraph::new(cx.text, shown, self.style.clone())),
        };
        let text = paragraph.wrap(cx.text, f32::INFINITY);
        let fill = if avail.width.is_finite() { avail.width } else { text.width };
        Size::new(known.width.unwrap_or(fill), self.style.line_px())
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        let (_, caret_index, composed, selected) = self.shown();
        let Some(paragraph) = &self.paragraph else { return };
        let (rect, caret) = (cx.rect, paragraph.x_at(caret_index));
        // The caret stays in view, and the text never leaves a gap it could fill.
        let room = (rect.width - 1.0).max(0.0);
        self.scroll_x = self.scroll_x.clamp((caret - room).max(0.0), caret);
        self.scroll_x = self.scroll_x.min((paragraph.size().width - room).max(0.0));
        let left = rect.x - self.scroll_x;
        let (outer, inner) = (cx.clip, cx.clip.then(cx.device(rect), 0.0));
        cx.clip = inner;

        if !composed.is_empty() {
            // What is being composed is underlined, as input methods show it.
            let (from, to) = (paragraph.x_at(composed.start), paragraph.x_at(composed.end));
            let bounds = Rect::new(left + from, rect.bottom() - 1.0, to - from, 1.0);
            cx.quad(Quad { bounds, background: self.color, ..Quad::default() });
        } else if !selected.is_empty() {
            let (from, to) = (paragraph.x_at(selected.start), paragraph.x_at(selected.end));
            let bounds = Rect::new(left + from, rect.y, to - from, rect.height);
            cx.quad(Quad { bounds, background: self.selection, ..Quad::default() });
        }
        self.tiles.clear();
        paint_glyphs(cx, paragraph, Point::new(left, rect.y), self.color, &mut self.tiles);
        if self.focused {
            // One device pixel wide at least, on a whole pixel: a caret is never blurred.
            let x = ((left + caret) * cx.scale).round() / cx.scale;
            let width = cx.scale.round().max(1.0) / cx.scale;
            let bounds = cx.device(Rect::new(x, rect.y, width, rect.height));
            // Always one instance, clear while the blink hides it: a blink overwrites it in
            // place and moves nothing else of the layer.
            let background = if self.caret_on { self.color } else { Color::TRANSPARENT };
            cx.push(Quad { bounds, background, ..Quad::default() }.instance(&cx.clip));
        }
        cx.clip = outer;
        self.caret_at = Some(Rect::new(left + caret - rect.x, 0.0, 1.0, rect.height));
    }

    fn caret(&self) -> Option<Rect> {
        self.caret_at
    }

    fn semantics(&self, says: &mut Semantics) {
        if says.role == Role::None {
            says.role = Role::TextInput;
        }
        says.value = Some(self.shown().0.into_owned()); // of a password, the dots
    }
}
