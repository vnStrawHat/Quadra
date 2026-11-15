//! The editor's widget (specs/13, "Code"): the document, its highlighter and what finding
//! found; measured as high as its lines, and where things are in it.

use std::ops::Range;

use quadra_a11y::{Role, Semantics};
use quadra_kit::token;
use quadra_layout::Known;
use quadra_prim::{Color, Rect, Size};
use quadra_reactive::Signal;
use quadra_text::TextStyle;
use quadra_tree::{GlyphTile, Inherited, MeasureCx, PaintCx, Theme, Widget};

use super::marks::Mark;
use super::rows::Rows;
use crate::doc::Doc;
use crate::doc::motion::columns;
use crate::find::{Query, next_match};
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
    /// What is looked for while the find bar is open, where it was found, and the match
    /// that is chosen.
    pub find: Option<Query>,
    pub matches: Vec<Range<usize>>,
    /// The version of the text the matches are of.
    pub(super) matched: u64,
    pub current: Option<usize>,
    /// Told the place of the chosen match and the count, or `None` for a bad pattern.
    pub(super) found: Option<Signal<Option<(usize, usize)>>>,
    /// What an input method is composing at the primary cursor, and its caret in it.
    pub preedit: Option<(String, usize)>,
    /// The clause of it being converted, as bytes of it: underlined thicker. Empty: none.
    pub clause: Range<usize>,
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
    /// The text the marks of the gutter compare with, the marks, and their colours.
    pub(super) base: Option<super::marks::Base>,
    pub marks: Vec<Mark>,
    pub(super) mark_colors: [Color; 3],
    /// The link under the pointer, and whether Ctrl is held over it.
    pub(super) link: Option<(Range<usize>, String)>,
    pub(super) link_hot: bool,
    /// It holds prose (plain text, Markdown): what no token covers is the editor's text
    /// colour, not a variable's.
    pub prose: bool,
    /// Lines are broken at the text's width, into the rows that `rows` holds.
    pub wrap: bool,
    pub(super) rows: Option<Rows>,
    pub(super) tiles: Vec<GlyphTile>,
    pub(super) tokens: Vec<Token>,
}

impl EditorWidget {
    pub fn new(text: &str, highlight: Box<dyn Highlight>) -> EditorWidget {
        EditorWidget {
            doc: Doc::new(text),
            highlight,
            find: None,
            matches: Vec::new(),
            matched: 0,
            current: None,
            found: None,
            preedit: None,
            clause: 0..0,
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
            base: None,
            marks: Vec::new(),
            mark_colors: [Color::TRANSPARENT; 3],
            link: None,
            link_hot: false,
            prose: false,
            wrap: false,
            rows: None,
            tiles: Vec::new(),
            tokens: Vec::new(),
        }
    }

    /// The width of the text, right of the gutter.
    pub(super) fn text_width(&self) -> f32 {
        (self.width - self.gutter).max(0.0)
    }

    /// Finds the query again in the text as it is, chooses the match at or after the
    /// primary cursor, and tells the bar.
    pub(super) fn refind(&mut self) {
        let found = match &self.find {
            Some(query) => query.matches(&self.doc),
            None => Ok(Vec::new()),
        };
        self.chosen(found);
    }

    /// After the text changed: the lines of the changes are searched again, when the query
    /// lets that be; all of it otherwise.
    pub(super) fn refind_changed(&mut self) {
        let kept = std::mem::take(&mut self.matches);
        match self.find.as_ref().and_then(|query| query.rematch(&self.doc, self.matched, kept)) {
            Some(matches) => self.chosen(Ok(matches)),
            None => self.refind(),
        }
    }

    /// Takes the matches found, chooses the one at or after the primary cursor, and tells
    /// the bar.
    fn chosen(&mut self, found: Result<Vec<Range<usize>>, String>) {
        self.matched = self.doc.version;
        let told = found.as_ref().ok().map(|m| (0, m.len()));
        self.matches = found.unwrap_or_default();
        self.current = next_match(&self.matches, self.doc.primary().start(), false);
        self.tell(told.map(|(_, n)| (self.current.map_or(0, |i| i + 1), n)));
    }

    fn tell(&self, what: Option<(usize, usize)>) {
        if let Some(found) = self.found
            && found.get_untracked() != what
        {
            found.set(what);
        }
    }

    /// Selects the next match after the primary cursor, or the one before it.
    pub(super) fn go_to_match(&mut self, back: bool) {
        let from = if back { self.doc.primary().start() } else { self.doc.primary().end() };
        let from = if back { from } else { from.max(self.doc.primary().start() + 1) };
        let Some(i) = next_match(&self.matches, from, back) else { return };
        let m = self.matches[i].clone();
        self.doc.set_cursor(m.start, m.end, false);
        self.current = Some(i);
        self.tell(Some((i + 1, self.matches.len())));
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

    /// As wide as the view, as high as the rows: the lines, or what they wrap into.
    fn measure(&mut self, cx: &mut MeasureCx, known: Known, avail: Size) -> Size {
        let face = cx.text.face_for(&self.style);
        self.cell = cx.text.word("M", face, self.style.size).width.max(1.0);
        self.line_px = self.style.line_px().max(1.0);
        let lines = self.doc.buffer.line_count();
        let digits = lines.to_string().len().max(4);
        self.gutter = (digits + 5) as f32 * self.cell;
        self.width = known.width.unwrap_or(if avail.width.is_finite() { avail.width } else { 0.0 });
        self.sync_rows();
        Size::new(self.width, self.row_count() as f32 * self.line_px)
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
            text: theme.color(if self.prose { token::EDITOR_FOREGROUND } else { token::SYNTAX_VARIABLE }),
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
        let marks = super::marks::colors(theme);
        let marked = std::mem::replace(&mut self.mark_colors, marks) != marks;
        (std::mem::replace(&mut self.colors, colors) != colors) | marked
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
