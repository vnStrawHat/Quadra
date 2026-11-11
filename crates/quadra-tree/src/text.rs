//! The text widget: a node that owns a paragraph (specs/08, "Paragraphs").

use quadra_layout::Known;
use quadra_prim::{Color, Corners, Rect, Size};
use quadra_render::{Kind, Sprite};
use quadra_text::{Paragraph, TextStyle};

use crate::fonts::{GlyphTile, SUBPIXEL_STEPS};
use crate::widget::{MeasureCx, PaintCx, Widget};

pub struct Text {
    text: String,
    style: TextStyle,
    pub color: Color,
    /// Shaped at the first measure, when the text system is at hand, and again only when the
    /// text or the style changed.
    paragraph: Option<Paragraph>,
    stale: bool,
    /// The tiles of the glyphs last painted. Holding them keeps them in the atlas: the
    /// instances in the layer point at their rectangles.
    tiles: Vec<GlyphTile>,
}

impl Text {
    pub fn new(text: impl Into<String>, style: TextStyle, color: Color) -> Text {
        Text { text: text.into(), style, color, paragraph: None, stale: false, tiles: Vec::new() }
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
        self.stale = true;
        true
    }

    /// Replaces the shaping style; false when it is equal.
    pub fn set_style(&mut self, style: TextStyle) -> bool {
        if self.style == style {
            return false;
        }
        self.style = style;
        self.stale = true;
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
            none => none.insert(Paragraph::new(cx.text, self.text.as_str(), self.style.clone())),
        };
        paragraph.wrap(cx.text, known.width.unwrap_or(avail.width))
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        let Some(paragraph) = &self.paragraph else { return };
        self.tiles.clear();
        let (ascent, descent) = paragraph.metrics();
        let (size, line_px, scale) = (paragraph.style().size, paragraph.style().line_px(), cx.scale);
        for line in paragraph.lines() {
            // Baselines sit on whole pixels; along the line a glyph keeps a quarter-pixel phase.
            let baseline =
                ((cx.rect.y + line.y + (line_px - ascent - descent) / 2.0 + ascent) * scale).round();
            let (glyphs, origin) = paragraph.line_glyphs(line);
            for g in glyphs {
                let x = (cx.rect.x + g.x - origin) * scale;
                let phase = ((x - x.floor()) * SUBPIXEL_STEPS as f32) as u8;
                let Some(tile) = cx.fonts.glyph(g.face, g.id, size * scale, phase) else { continue };
                if tile.rect.w == 0 {
                    continue; // no ink: a space
                }
                let (left, top) =
                    (x.floor() + tile.meta.left, baseline - (g.y * scale).round() + tile.meta.top);
                let sprite = Sprite {
                    bounds: Rect::new(left, top, tile.rect.w as f32, tile.rect.h as f32),
                    uv: tile.uv,
                    slice: tile.rect.slice,
                    color: self.color,
                    radii: Corners::default(),
                    glyph: true,
                };
                let instance =
                    sprite.instance(if tile.meta.color { Kind::Poly } else { Kind::Mono }, &cx.clip);
                cx.push(instance);
                self.tiles.push(tile);
            }
        }
    }
}
