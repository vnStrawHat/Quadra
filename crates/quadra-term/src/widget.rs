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

use crate::session::Session;

/// Padding around the grid, in logical pixels.
pub(crate) const PADDING: f32 = 8.0;

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
    pub focused: bool,
    pub caret_on: bool,
    pub(crate) tiles: Vec<GlyphTile>,
}

pub(crate) fn color(rgb: Rgb) -> Color {
    Color::rgb(u32::from(rgb.r) << 16 | u32::from(rgb.g) << 8 | u32::from(rgb.b))
}

fn rgb(c: Color) -> Rgb {
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
            focused: false,
            caret_on: true,
            tiles: Vec::new(),
        }
    }

    /// The grid that fits a box of `size`: rows and columns, at least one of each.
    pub(crate) fn grid_for(&self, size: Size) -> (u16, u16) {
        let fit = |room: f32, step: f32| ((room - 2.0 * PADDING) / step).floor().clamp(1.0, 32767.0) as u16;
        (fit(size.height, self.line), fit(size.width, self.cell))
    }

    /// Takes the engine's snapshot (the first phase of drawing, under the lock): a paint
    /// reads it alone, so a paint from scratch draws what the last one did.
    pub(crate) fn refresh(&mut self) {
        let mut engine = self.session.engine();
        // ponytail: a `?2026` block held past its deadline waits for the next output or key
        // to be shown; a timer at `sync().deadline()` when a program leaves one open.
        engine.term.snapshot_update(&mut self.state, Instant::now());
        self.palette = engine.term.palette();
        self.cursor_style = engine.term.cursor_style();
        self.modes = engine.term.mode_snapshot();
    }

    /// The glyphs the last paint holds.
    pub fn tiles_painted(&self) -> usize {
        self.tiles.len()
    }
}

/// Takes the snapshot of a terminal's widget and has it painted again.
pub(crate) fn repaint(tree: &mut Tree, id: NodeId) {
    if let Some(w) = tree.widget_mut::<TerminalWidget>(id) {
        w.refresh();
    }
    tree.mark_paint(id);
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
        self.session.engine().term.set_theme_colors(&palette);
        self.refresh();
        self.selection = theme.color(token::EDITOR_SELECTION);
        true
    }

    /// The cursor's cell: an input method opens beside it.
    fn caret(&self) -> Option<Rect> {
        let cursor = self.state.cursor();
        let row = cursor.row.filter(|_| self.focused)?;
        let (x, y) = (PADDING + f32::from(cursor.col) * self.cell, PADDING + f32::from(row) * self.line);
        Some(Rect::new(x, y, self.cell, self.line))
    }

    /// A reader is told the rows that show.
    fn semantics(&self, says: &mut Semantics) {
        if says.role == Role::None {
            says.role = Role::TextInput;
        }
        says.value = Some(self.session.screen_text().join("\n").trim_end().to_string());
    }
}
