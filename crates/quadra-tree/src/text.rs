//! The text widget: a node that owns a paragraph (specs/08, "Paragraphs"). Its style is
//! not its own: it is laid out and painted with the text values that hold at its node.

use quadra_a11y::{Role, Semantics};
use quadra_layout::{Holds, Known};
use quadra_prim::{Color, Corners, Point, Rect, Size};
use quadra_render::{Kind, Quad, Sprite};
use quadra_text::{EditBuffer, FaceId, GlyphFormat, Paragraph, TextStyle, Word};

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
    /// One line, cut where its box ends with an ellipsis, shaped at measure.
    pub ellipsis: Option<Option<std::rc::Rc<Word>>>,
    /// The available widths the last measure's size holds for; the height it never reads.
    holds: Option<(f32, f32)>,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Text {
        let (text, style, color) = (text.into(), TextStyle::default(), Color::BLACK);
        let (selected, selection) = ((0, 0), Color::TRANSPARENT);
        let (tiles, ellipsis) = (Vec::new(), None);
        let (paragraph, stale, holds) = (None, false, None);
        Text { text, style, color, selection, paragraph, stale, selected, tiles, ellipsis, holds }
    }

    /// The offset in the text nearest a point from the node's corner.
    pub fn index_at(&self, at: Point) -> usize {
        let Some(lines) = &self.paragraph else { return 0 };
        lines.index_near(lines.line_at_y(at.y), at.x)
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
        let width = known.width.unwrap_or(avail.width);
        // A known width is the only one read.
        let any = known.width.map(|_| (0.0, f32::INFINITY));
        let Some(dots) = &mut self.ellipsis else {
            let size = paragraph.wrap(cx.text, width);
            self.holds = any.or(Some(paragraph.kept_widths()));
            return size;
        };
        // One line, as wide as the box lets it be: what is past its end is cut.
        let face = cx.text.face_for(&self.style);
        *dots = Some(cx.text.word("\u{2026}", face, self.style.size));
        let whole = paragraph.wrap(cx.text, f32::INFINITY);
        self.holds = any.or((whole.width <= width).then_some((whole.width, f32::INFINITY)));
        Size::new(whole.width.min(width), whole.height)
    }

    fn holds(&self) -> Option<Holds> {
        self.holds.map(|width| Holds { width, any_height: true })
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        let Some(paragraph) = &self.paragraph else { return };
        let (from, to) = (self.selected.0.min(self.selected.1), self.selected.0.max(self.selected.1));
        for span in paragraph.spans(from..to, self.style.size * 0.3) {
            let bounds = span.translate(cx.rect.x, cx.rect.y);
            cx.quad(Quad { bounds, background: self.selection, ..Quad::default() });
        }
        self.tiles.clear();
        match &self.ellipsis {
            Some(Some(dots)) if paragraph.size().width > cx.rect.width + 0.5 => {
                cut(cx, paragraph, dots, self.color, &mut self.tiles)
            }
            _ => paint_glyphs(cx, paragraph, cx.rect.origin(), self.color, &mut self.tiles),
        }
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

/// Paints the one line of a paragraph as far as there is room for it and an ellipsis after
/// it, at the node's corner.
fn cut(cx: &mut PaintCx, paragraph: &Paragraph, dots: &Word, color: Color, tiles: &mut Vec<GlyphTile>) {
    let Some(line) = paragraph.lines().first() else { return };
    let (scale, origin) = (cx.scale, cx.rect.origin());
    let baseline = ((origin.y + line.baseline) * scale).round();
    let (glyphs, start) = paragraph.line_glyphs(line);
    let room = cx.rect.width - dots.width;
    let mut end = 0.0f32;
    for g in glyphs.iter().filter(|g| g.x - start + g.advance <= room) {
        let (_, size) = paragraph.run_at(g.cluster as usize);
        let pen = Point::new((origin.x + g.x - start) * scale, baseline - (g.y * scale).round());
        tiles.extend(paint_glyph(cx, (g.face, g.id), size * scale, pen, color));
        end = end.max(g.x - start + g.advance);
    }
    for g in &dots.glyphs {
        let pen = Point::new((origin.x + end + g.x) * scale, baseline - (g.y * scale).round());
        tiles.extend(paint_glyph(cx, (g.face, g.id), paragraph.style().size * scale, pen, color));
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
    paint_runs(cx, paragraph, origin, |_| color, tiles);
}

/// The same for a paragraph of runs: `color` gives the colour of each, by its number.
pub(crate) fn paint_runs(
    cx: &mut PaintCx,
    paragraph: &Paragraph,
    origin: Point,
    color: impl Fn(usize) -> Color,
    tiles: &mut Vec<GlyphTile>,
) {
    let scale = cx.scale;
    for line in paragraph.lines() {
        let top = origin.y + line.y;
        if cx.visible.is_some_and(|band| top > band.bottom() || top + line.height < band.y) {
            continue; // outside what its layer paints
        }
        // Baselines sit on whole pixels; along the line a glyph keeps a quarter-pixel phase.
        let baseline = ((top + line.baseline) * scale).round();
        let (glyphs, start) = paragraph.line_glyphs(line);
        for g in glyphs {
            let x = (origin.x + g.x - start) * scale;
            let (run, size) = paragraph.run_at(g.cluster as usize);
            let pen = Point::new(x, baseline - (g.y * scale).round());
            tiles.extend(paint_glyph(cx, (g.face, g.id), size * scale, pen, color(run)));
        }
    }
}

/// Paints a glyph of a face at a size, both in device pixels, its pen at `pen`: along the
/// line a glyph keeps a quarter-pixel phase, and the baseline is on a whole pixel. Gives the
/// tile to hold, which keeps it in the atlas; none for a glyph with no ink.
pub fn paint_glyph(
    cx: &mut PaintCx,
    glyph: (FaceId, u16),
    size: f32,
    pen: Point,
    color: Color,
) -> Option<GlyphTile> {
    paint_glyph_with(cx, glyph, size, pen, color, false)
}

/// [`paint_glyph`], smoothed by pixel when `grayscale` even where the OS smooths by subpixel.
pub fn paint_glyph_with(
    cx: &mut PaintCx,
    (face, id): (FaceId, u16),
    size: f32,
    pen: Point,
    color: Color,
    grayscale: bool,
) -> Option<GlyphTile> {
    let phase = ((pen.x - pen.x.floor()) * SUBPIXEL_STEPS as f32) as u8;
    let phase = if grayscale { phase | quadra_text::GRAYSCALE } else { phase };
    let tile = cx.fonts.glyph(face, id, size, phase).filter(|tile| tile.rect.w > 0)?;
    let (left, top) = (pen.x.floor() + tile.meta.left, pen.y + tile.meta.top);
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
    cx.push(sprite.instance(if coverage { Kind::Mono } else { Kind::Poly }, &cx.clip));
    Some(tile)
}
