//! The editor's widget (specs/13, "Code"): the document, its highlighter and what finding
//! found; measured as high as its lines, and where things are in it.

use std::ops::Range;

use quadra_a11y::{Role, Semantics};
use quadra_kit::token;
use quadra_layout::Known;
use quadra_prim::{Color, Point, Rect, Size};
use quadra_text::TextStyle;
use quadra_tree::{GlyphTile, Inherited, MeasureCx, PaintCx, Theme, Widget};

use crate::doc::Doc;
use crate::doc::motion::columns;
use crate::highlight::Highlight;
use crate::token::Token;

/// The colours of the editor, from roles of the theme.
#[derive(Clone, Copy, Default, PartialEq)]
pub(super) struct Colors {
    pub text: Color,
    pub cursor: Color,
    pub line: Color,
    pub gutter: Color,
    pub gutter_active: Color,
    pub selection: Color,
    pub find: Color,
    pub find_current: Color,
    pub bracket: Color,
    pub syntax: [Color; 17],
}

pub struct EditorWidget {
    pub doc: Doc,
    /// What colours its lines.
    pub highlight: Box<dyn Highlight>,
    /// Where finding found the query, and the match that is chosen.
    pub matches: Vec<Range<usize>>,
    pub current: Option<usize>,
    /// What an input method is composing at the primary cursor, and its caret in it.
    pub preedit: Option<(String, usize)>,
    pub focused: bool,
    pub caret_on: bool,
    /// How far the text is scrolled sideways.
    pub scroll_x: f32,
    pub(super) style: TextStyle,
    pub(super) colors: Colors,
    /// The width of a cell and the height of a line, from the last measure.
    pub(super) cell: f32,
    pub(super) line_px: f32,
    pub(super) gutter: f32,
    pub(super) width: f32,
    /// The widest line painted so far: how far it scrolls sideways.
    pub(super) widest: f32,
    pub(super) tiles: Vec<GlyphTile>,
    pub(super) tokens: Vec<Token>,
}

impl EditorWidget {
    pub fn new(text: &str, highlight: Box<dyn Highlight>) -> EditorWidget {
        EditorWidget {
            doc: Doc::new(text),
            highlight,
            matches: Vec::new(),
            current: None,
            preedit: None,
            focused: false,
            caret_on: true,
            scroll_x: 0.0,
            style: TextStyle::default(),
            colors: Colors::default(),
            cell: 8.0,
            line_px: 20.0,
            gutter: 0.0,
            width: 0.0,
            widest: 0.0,
            tiles: Vec::new(),
            tokens: Vec::new(),
        }
    }

    /// The width of the text, right of the gutter.
    pub(super) fn text_width(&self) -> f32 {
        (self.width - self.gutter).max(0.0)
    }

    /// The x of an offset from the left of the text, before scrolling.
    pub(super) fn x_of(&self, offset: usize) -> f32 {
        self.doc.col_of(offset) as f32 * self.cell
    }

    /// The caret of the primary cursor, from the node's corner.
    pub fn caret_rect(&self) -> Rect {
        let head = self.doc.primary().head;
        let line = self.doc.buffer.line_of(head);
        let x = self.gutter + self.x_of(head) - self.scroll_x;
        Rect::new(x, line as f32 * self.line_px, 2.0, self.line_px)
    }

    /// The offset nearest a point from the node's corner.
    pub fn offset_at(&self, at: Point) -> usize {
        let last = self.doc.buffer.line_count() - 1;
        let line = ((at.y / self.line_px).floor().max(0.0) as usize).min(last);
        self.doc.offset_at_col(line, self.col_at(at.x))
    }

    /// The column nearest an x from the node's corner.
    pub(super) fn col_at(&self, x: f32) -> usize {
        ((x - self.gutter + self.scroll_x) / self.cell).round().max(0.0) as usize
    }

    /// Scrolls sideways as little as keeps the caret in view, four cells from the edges.
    pub fn follow_caret(&mut self) {
        let (x, room, margin) = (self.x_of(self.doc.primary().head), self.text_width(), 4.0 * self.cell);
        self.widest = self.widest.max(x + margin);
        if x < self.scroll_x + margin {
            self.scroll_x = (x - margin).max(0.0);
        } else if x > self.scroll_x + room - margin {
            self.scroll_x = x - room + margin;
        }
    }

    /// Scrolls sideways by `dx`, as far as the widest line painted.
    pub fn scroll_by(&mut self, dx: f32) -> bool {
        let to = (self.scroll_x + dx).clamp(0.0, (self.widest - self.text_width()).max(0.0));
        std::mem::replace(&mut self.scroll_x, to) != to
    }

    /// The glyphs the last paint holds: those of the lines that showed.
    pub fn tiles_painted(&self) -> usize {
        self.tiles.len()
    }

    /// The columns of a line's text, its tabs at their stops.
    pub(super) fn columns(&self, text: &str) -> usize {
        columns(text, self.doc.tab_size.max(1))
    }
}

impl Widget for EditorWidget {
    fn is_leaf(&self) -> bool {
        true
    }

    /// As wide as the view, as high as the lines.
    fn measure(&mut self, cx: &mut MeasureCx, known: Known, avail: Size) -> Size {
        let face = cx.text.face_for(&self.style);
        self.cell = cx.text.word("M", face, self.style.size).width.max(1.0);
        self.line_px = self.style.line_px().max(1.0);
        let lines = self.doc.buffer.line_count();
        let digits = lines.to_string().len().max(4);
        self.gutter = (digits + 5) as f32 * self.cell;
        self.width = known.width.unwrap_or(if avail.width.is_finite() { avail.width } else { 0.0 });
        Size::new(self.width, lines as f32 * self.line_px)
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        super::paint::paint(self, cx);
    }

    fn inherit(&mut self, inherited: &Inherited) {
        self.style.clone_from(&inherited.style);
    }

    fn theme(&mut self, theme: &Theme) -> bool {
        let syntax = crate::syntax::KINDS.map(|kind| theme.color(crate::syntax::role(kind)));
        let colors = Colors {
            text: theme.color(token::SYNTAX_VARIABLE),
            cursor: theme.color(token::EDITOR_CURSOR),
            line: theme.color(token::EDITOR_LINE),
            gutter: theme.color(token::EDITOR_GUTTER),
            gutter_active: theme.color(token::EDITOR_GUTTER_ACTIVE),
            selection: theme.color(token::EDITOR_SELECTION),
            find: theme.color(token::EDITOR_FIND),
            find_current: theme.color(token::EDITOR_FIND_CURRENT),
            bracket: theme.color(token::EDITOR_BRACKET),
            syntax,
        };
        std::mem::replace(&mut self.colors, colors) != colors
    }

    fn caret(&self) -> Option<Rect> {
        self.focused.then(|| self.caret_rect())
    }

    /// A text input of many lines: its text, and its selection in characters.
    // ponytail: the whole text a change; a window of it if large files make readers slow.
    fn semantics(&self, says: &mut Semantics) {
        if says.role == Role::None {
            says.role = Role::TextInput;
        }
        let text = self.doc.text();
        let primary = self.doc.primary();
        let at = |offset: usize| text[..offset.min(text.len())].chars().count();
        says.selection = Some((at(primary.anchor), at(primary.head)));
        says.value = Some(text);
    }
}
