//! The text widget: a node that owns a paragraph (specs/08, "Paragraphs"). Its style is
//! not its own: it is laid out and painted with the text values that hold at its node.

use quadra_a11y::{Role, Semantics};
use quadra_layout::Known;
use quadra_prim::{Color, Corners, Point, Rect, Size};
use quadra_render::{Kind, Quad, Sprite};
use quadra_text::{EditBuffer, GlyphFormat, Paragraph, TextStyle};

use crate::fonts::{GlyphTile, SUBPIXEL_STEPS};
use crate::inherit::Inherited;
use crate::widget::{MeasureCx, PaintCx, Widget};

pub struct Text {
    text: String,
    /// The style and the colours that hold at its node.
    style: TextStyle,
    color: Color,
    selection: Color,
    /// Shaped at the first measure, when the text system is at hand, and again only when the
    /// text or the style changed.
    paragraph: Option<Paragraph>,
    stale: bool,
    /// What is selected, in a text that can be: from where the selection started to where
    /// it was taken, in either order. Equal: nothing is.
    selected: (usize, usize),
    /// The tiles of the glyphs last painted. Holding them keeps them in the atlas: the
    /// instances in the layer point at their rectangles.
    tiles: Vec<GlyphTile>,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Text {
        let (text, style, color) = (text.into(), TextStyle::default(), Color::BLACK);
        let (selected, selection) = ((0, 0), Color::TRANSPARENT);
        Text { text, style, color, selection, paragraph: None, stale: false, selected, tiles: Vec::new() }
    }

    /// The offset in the text nearest a point from the node's corner.
    pub fn index_at(&self, at: Point) -> usize {
        let Some(lines) = &self.paragraph else { return 0 };
        let line = (at.y / self.style.line_px()).floor().max(0.0) as usize;
        lines.index_near(line.min(lines.lines().len().saturating_sub(1)), at.x)
    }

    /// Starts a selection at an offset, or with `extend` takes the one there is to it.
    pub fn select_to(&mut self, index: usize, extend: bool) {
        let index = (0..=index.min(self.text.len())).rev().find(|&i| self.text.is_char_boundary(i));
        self.selected.1 = index.unwrap_or(0);
        if !extend {
            self.selected.0 = self.selected.1;
        }
    }

    /// Selects the word around an offset, or with `None` everything.
    pub fn select_word(&mut self, around: Option<usize>) {
        self.selected = (0, self.text.len());
        if let Some(index) = around {
            // ponytail: a copy of the text for one double click; the word rules are the
            // edit buffer's.
            let mut words = EditBuffer::new(self.text.as_str());
            words.select_word(index);
            self.selected = (words.selection().start, words.selection().end);
        }
    }

    /// The text that is selected.
    pub fn selected(&self) -> &str {
        let (from, to) = (self.selected.0.min(self.selected.1), self.selected.0.max(self.selected.1));
        &self.text[from..to]
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the text; false when it is equal.
    pub fn set_text(&mut self, text: &str) -> bool {
        if self.text == text {
            return false;
        }
        self.text.clear();
        self.text.push_str(text);
        (self.stale, self.selected) = (true, (0, 0)); // what was selected is not there any more
        true
    }
}

impl Widget for Text {
    fn is_leaf(&self) -> bool {
        true
    }

    fn measure(&mut self, cx: &mut MeasureCx, known: Known, avail: Size) -> Size {
        let paragraph = match &mut self.paragraph {
            Some(paragraph) => {
                if std::mem::take(&mut self.stale) {
                    paragraph.set_text(cx.text, &self.text);
                    paragraph.set_style(cx.text, &self.style);
                }
                paragraph
            }
            none => {
                self.stale = false;
                none.insert(Paragraph::new(cx.text, self.text.as_str(), self.style.clone()))
            }
        };
        paragraph.wrap(cx.text, known.width.unwrap_or(avail.width))
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        let Some(paragraph) = &self.paragraph else { return };
        let (from, to) = (self.selected.0.min(self.selected.1), self.selected.0.max(self.selected.1));
        for span in paragraph.spans(from..to, self.style.size * 0.3) {
            let bounds = span.translate(cx.rect.x, cx.rect.y);
            cx.quad(Quad { bounds, background: self.selection, ..Quad::default() });
        }
        self.tiles.clear();
        paint_glyphs(cx, paragraph, cx.rect.origin(), self.color, &mut self.tiles);
    }

    fn inherit(&mut self, inherited: &Inherited) {
        (self.color, self.selection) = (inherited.color, inherited.selection);
        if self.style != inherited.style {
            self.style.clone_from(&inherited.style);
            self.stale = true;
        }
    }

    fn semantics(&self, says: &mut Semantics) {
        if says.role == Role::None {
            says.role = Role::Text;
        }
        if says.name.is_empty() {
            says.name.clone_from(&self.text);
        }
    }
}

/// Paints the lines of a paragraph whose top-left corner is at `origin`, in logical pixels of
/// the layer, and keeps the tiles of what it painted in `tiles`. Lines outside the part of
/// the layer that is painted are skipped.
pub(crate) fn paint_glyphs(
    cx: &mut PaintCx,
    paragraph: &Paragraph,
    origin: Point,
    color: Color,
    tiles: &mut Vec<GlyphTile>,
) {
    let (ascent, descent) = paragraph.metrics();
    let (size, line_px, scale) = (paragraph.style().size, paragraph.style().line_px(), cx.scale);
    for line in paragraph.lines() {
        let top = origin.y + line.y;
        if cx.visible.is_some_and(|band| top > band.bottom() || top + line_px < band.y) {
            continue; // outside what its layer paints
        }
        // Baselines sit on whole pixels; along the line a glyph keeps a quarter-pixel phase.
        let baseline = ((top + (line_px - ascent - descent) / 2.0 + ascent) * scale).round();
        let (glyphs, start) = paragraph.line_glyphs(line);
        for g in glyphs {
            let x = (origin.x + g.x - start) * scale;
            let phase = ((x - x.floor()) * SUBPIXEL_STEPS as f32) as u8;
            let Some(tile) = cx.fonts.glyph(g.face, g.id, size * scale, phase) else { continue };
            if tile.rect.w == 0 {
                continue; // no ink: a space
            }
            let (left, top) = (x.floor() + tile.meta.left, baseline - (g.y * scale).round() + tile.meta.top);
            let sprite = Sprite {
                bounds: Rect::new(left, top, tile.rect.w as f32, tile.rect.h as f32),
                uv: tile.uv,
                slice: tile.rect.slice,
                color,
                radii: Corners::default(),
                // A colour glyph is an image: its pixels are not coverage of the text colour.
                glyph: tile.meta.format != GlyphFormat::Color,
            };
            let coverage = tile.meta.format == GlyphFormat::Coverage;
            let instance = sprite.instance(if coverage { Kind::Mono } else { Kind::Poly }, &cx.clip);
            cx.push(instance);
            tiles.push(tile);
        }
    }
}
