//! The terminal's widget (specs/18, "The view"): it fills its box, sizes its cells from the
//! mono face, takes the engine's snapshot when it paints and draws the rows that show.

use std::time::Instant;

use oneterm_vt::{CursorStyle, ModeSnapshot, Palette, Rgb, SnapshotState};
use quadra_a11y::{Role, Semantics};
use quadra_design::token;
use quadra_layout::Known;
use quadra_prim::{Color, Rect, Size};
use quadra_text::TextStyle;
use quadra_tree::{GlyphTile, Inherited, MeasureCx, NodeId, PaintCx, Theme, Tree, Widget};

use crate::feed::Engine;
use crate::look::TermColors;
use crate::session::Session;

pub struct TerminalWidget {
    pub(crate) session: Session,
    pub(crate) state: SnapshotState,
    pub(crate) palette: Palette,
    pub(crate) cursor_style: CursorStyle,
    pub(crate) modes: ModeSnapshot,
    pub(crate) style: TextStyle,
    /// A cell's width and height, whole device pixels at `scale`.
    pub(crate) cell: f32,
    pub(crate) line: f32,
    pub(crate) scale: f32,
    pub(crate) selection: Color,
    /// Room around the grid, in logical pixels.
    pub(crate) pad: f32,
    /// The look's colours, over the theme's palette and selection.
    pub(crate) colors: TermColors,
    base: (Palette, Color),
    pub focused: bool,
    pub caret_on: bool,
    pub(crate) tiles: Vec<GlyphTile>,
    /// The gutter: shown, its digits (they only grow), the line and time of each row that
    /// shows, the clock that turns instants into local times, and the label being drawn.
    pub(crate) gutter: bool,
    pub(crate) digits: usize,
    pub(crate) gutter_rows: Vec<Option<(u64, Option<Instant>)>>,
    pub(crate) clock: (Instant, i64),
    pub(crate) label: String,
}

pub(crate) fn color(rgb: Rgb) -> Color {
    Color::rgb(u32::from(rgb.r) << 16 | u32::from(rgb.g) << 8 | u32::from(rgb.b))
}

pub(crate) fn rgb(c: Color) -> Rgb {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Rgb { r: byte(c.r), g: byte(c.g), b: byte(c.b) }
}

impl TerminalWidget {
    pub(crate) fn new(session: Session) -> TerminalWidget {
        TerminalWidget {
            session,
            state: SnapshotState::new(),
            palette: Palette::new(),
            cursor_style: CursorStyle::default(),
            modes: ModeSnapshot::default(),
            style: TextStyle::default(),
            cell: 8.0,
            line: 16.0,
            scale: 1.0,
            selection: Color::TRANSPARENT,
            pad: 8.0,
            colors: TermColors::default(),
            base: (Palette::new(), Color::TRANSPARENT),
            focused: false,
            caret_on: true,
            tiles: Vec::new(),
            gutter: false,
            digits: crate::gutter::DIGITS,
            gutter_rows: Vec::new(),
            clock: (Instant::now(), 0),
            label: String::new(),
        }
    }

    /// The grid that fits a box of `size`: rows and columns, at least one of each.
    pub(crate) fn grid_for(&self, size: Size) -> (u16, u16) {
        let fit = |room: f32, step: f32| ((room - 2.0 * self.pad) / step).floor().clamp(1.0, 32767.0) as u16;
        (fit(size.height, self.line), fit(size.width - self.gutter_width(), self.cell))
    }

    /// Takes the engine's snapshot (the first phase of drawing, under the lock): a paint
    /// reads it alone, so a paint from scratch draws what the last one did. True when the
    /// gutter needs more digits, so the grid is sized again.
    pub(crate) fn refresh(&mut self) -> bool {
        let mut engine = self.session.engine();
        let Engine { term, stamps, .. } = &mut *engine;
        // ponytail: a `?2026` block held past its deadline waits for the next output or key
        // to be shown; a timer at `sync().deadline()` when a program leaves one open.
        term.snapshot_update(&mut self.state, Instant::now());
        self.palette = term.palette();
        self.cursor_style = term.cursor_style();
        self.modes = term.mode_snapshot();
        self.gutter_rows.clear();
        if !self.gutter {
            return false;
        }
        stamps.update(term, false, None);
        let alt = term.grid().alt_active();
        let rows = self.state.rows().iter().map(|row| (!alt).then(|| stamps.at(row.id)));
        self.gutter_rows.extend(rows);
        let most = self.gutter_rows.iter().flatten().map(|it| it.0).max().unwrap_or(0);
        let digits = crate::gutter::digits_of(most);
        (digits > self.digits).then(|| self.digits = digits).is_some()
    }

    /// The theme's colours with the look's over them, to the engine.
    pub(crate) fn paint_colors(&mut self) {
        let palette = self.colors.over(&self.base.0);
        self.session.engine().term.set_theme_colors(&palette);
        self.selection = self.colors.selection.unwrap_or(self.base.1);
        self.refresh();
    }

    /// The glyphs the last paint holds.
    pub fn tiles_painted(&self) -> usize {
        self.tiles.len()
    }
}

/// Takes the snapshot of a terminal's widget and has it painted again; sizes the grid again
/// when the gutter grew.
pub(crate) fn repaint(tree: &mut Tree, id: NodeId) {
    let grew = tree.widget_mut::<TerminalWidget>(id).and_then(|w| w.refresh().then(|| w.session.clone()));
    tree.mark_paint(id);
    if let Some(session) = grew {
        crate::view::regrid(tree, id, &session);
    }
    crate::line::follow_line(tree, id);
}

impl Widget for TerminalWidget {
    fn is_leaf(&self) -> bool {
        true
    }

    /// The box it is given; cells from the advance of `0` and the line, snapped.
    fn measure(&mut self, cx: &mut MeasureCx, known: Known, avail: Size) -> Size {
        let face = cx.text.face_for(&self.style);
        let snap = |v: f32| (v * self.scale).round().max(1.0) / self.scale;
        self.cell = snap(cx.text.word("0", face, self.style.size).width);
        self.line = snap(self.style.line_px());
        let fill =
            |known: Option<f32>, avail: f32| known.unwrap_or(if avail.is_finite() { avail } else { 0.0 });
        Size::new(fill(known.width, avail.width), fill(known.height, avail.height))
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        crate::paint::paint(self, cx);
    }

    fn inherit(&mut self, inherited: &Inherited) {
        self.style.clone_from(&inherited.style);
    }

    /// The engine's palette takes the theme as its base: 0-15 from its ANSI roles, the
    /// foreground, background and cursor from the editor's.
    fn theme(&mut self, theme: &Theme) -> bool {
        let mut palette = Palette::new();
        for (slot, role) in palette.indexed.iter_mut().zip(token::ANSI) {
            *slot = rgb(theme.color(role));
        }
        palette.foreground = rgb(theme.color(token::EDITOR_FOREGROUND));
        palette.background = rgb(theme.color(token::CANVAS));
        palette.cursor = rgb(theme.color(token::EDITOR_CURSOR));
        self.base = (palette, theme.color(token::EDITOR_SELECTION));
        self.paint_colors();
        true
    }

    /// The cursor's cell: an input method opens beside it.
    fn caret(&self) -> Option<Rect> {
        let cursor = self.state.cursor();
        let row = cursor.row.filter(|_| self.focused)?;
        let x = self.gutter_width() + self.pad + f32::from(cursor.col) * self.cell;
        let y = self.pad + f32::from(row) * self.line;
        Some(Rect::new(x, y, self.cell, self.line))
    }

    /// A reader is told the rows that show.
    fn semantics(&self, says: &mut Semantics) {
        if says.role == Role::None {
            says.role = Role::TextInput;
            says.name = "Terminal".into();
        }
        says.value = Some(self.session.screen_text().join("\n").trim_end().to_string());
    }
}
