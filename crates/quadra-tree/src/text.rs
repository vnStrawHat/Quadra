//! The text widget: a node that owns a paragraph (specs/08, "Paragraphs").

use quadra_layout::Known;
use quadra_prim::{Color, Corners, Rect, Size};
use quadra_render::{Kind, Sprite};
use quadra_text::{Paragraph, TextStyle};

use crate::widget::{MeasureCx, PaintCx, Widget};

pub struct Text {
    text: String,
    style: TextStyle,
    pub color: Color,
    /// Shaped at the first measure, when the text system is at hand, and again only when the
    /// text or the style changed.
    paragraph: Option<Paragraph>,
    stale: bool,
}

impl Text {
    pub fn new(text: impl Into<String>, style: TextStyle, color: Color) -> Text {
        Text { text: text.into(), style, color, paragraph: None, stale: false }
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
        let (ascent, descent) = paragraph.metrics();
        let (size, line_px) = (paragraph.style().size, paragraph.style().line_px());
        for line in paragraph.lines() {
            let baseline = cx.rect.y + line.y + (line_px - ascent - descent) / 2.0 + ascent;
            let (glyphs, origin) = paragraph.line_glyphs(line);
            for g in glyphs.iter().filter(|g| g.id != b' ' as u16 && g.id != b'\t' as u16) {
                // ponytail: every glyph is a box of half an em by 0.7 em, as the fake backend
                // rasterizes. Real tiles come with the atlases (specs/07, specs/08).
                let x = cx.rect.x + g.x - origin;
                let bounds = cx.device(Rect::new(x, baseline - size * 0.7 - g.y, size * 0.5, size * 0.7));
                let sprite = Sprite {
                    bounds,
                    uv: [0.0; 4],
                    slice: 0,
                    color: self.color,
                    radii: Corners::default(),
                    glyph: true,
                };
                let instance = sprite.instance(Kind::Mono, &cx.clip);
                cx.push(instance);
            }
        }
    }
}
