//! The rich text widget: one paragraph of spans, each with what it has otherwise than the
//! text values that hold at the node (specs/05, "Rules"; specs/08, "Paragraphs").

use std::cell::RefCell;
use std::rc::Rc;

use quadra_a11y::{Role, Semantics};
use quadra_layout::Known;
use quadra_prim::{Color, Point, Rect, Size};
use quadra_render::Quad;
use quadra_text::{Paragraph, Shaping, TextStyle};

use crate::fonts::GlyphTile;
use crate::inherit::Inherited;
use crate::text::paint_runs;
use crate::theme::{Paint, Theme};
use crate::widget::{MeasureCx, PaintCx, Widget};

/// A span of a rich text: where it ends in the text, and what it says of itself.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RichSpan {
    pub end: usize,
    pub shaping: Shaping,
    /// Its colour, where it is not that of the text around: a colour, or a role.
    pub color: Option<Paint>,
    /// A fill behind it, as high as its line.
    pub background: Option<Paint>,
    /// A line through it.
    pub strike: bool,
    /// What a click on it gives: it is a link, and underlined.
    pub link: Option<Rc<str>>,
}

/// What is in a line of rich text and is no text: a formula. Its span is one character,
/// U+FFFC, and as wide as the box says.
pub trait InlineBox {
    /// Its width, its height above the baseline and its depth below it, in a text of a
    /// style. A line it is on is made as high as it reaches.
    fn measure(&mut self, cx: &mut MeasureCx, style: &TextStyle) -> (f32, f32, f32);

    /// Paints it with the left end of its baseline at `origin`, in the colour of its span.
    fn paint(&mut self, cx: &mut PaintCx, origin: Point, color: Color);

    /// What it is to a reader: said in its place in the text.
    fn says(&self) -> &str;
}

/// Where the links of a rich text are, from the node's corner, as last painted: a box for
/// each line a link is on, and what the link gives. The view that handles the pointer holds
/// them too.
pub type LinkBoxes = Rc<RefCell<Vec<(Rect, Rc<str>)>>>;

pub struct RichText {
    text: String,
    /// In the order of the text; they end where the next starts.
    spans: Vec<RichSpan>,
    /// The colour and the fill of each span under the tree's theme.
    colors: Vec<(Option<Color>, Option<Color>)>,
    /// The style and the colour that hold at its node.
    style: TextStyle,
    color: Color,
    /// Shaped at the first measure, and again when the style changed.
    paragraph: Option<Paragraph>,
    stale: bool,
    links: LinkBoxes,
    tiles: Vec<GlyphTile>,
    /// What is in its lines and is no text, each with the number of its span.
    boxes: Vec<(usize, Box<dyn InlineBox>)>,
}

impl RichText {
    pub fn new(text: impl Into<String>, spans: Vec<RichSpan>) -> RichText {
        let (text, style, color) = (text.into(), TextStyle::default(), Color::BLACK);
        let (links, tiles) = (LinkBoxes::default(), Vec::new());
        let colors = Vec::new(); // the tree gives the theme when the node is added
        let boxes = Vec::new();
        RichText { text, spans, colors, style, color, paragraph: None, stale: false, links, tiles, boxes }
    }

    /// Puts a box in the place of a span, by the span's number.
    pub fn with_box(mut self, span: usize, inline: Box<dyn InlineBox>) -> Self {
        self.boxes.push((span, inline));
        self
    }

    /// Where a span starts and ends in the text.
    fn range(&self, span: usize) -> std::ops::Range<usize> {
        span.checked_sub(1).map_or(0, |before| self.spans[before].end)..self.spans[span].end
    }

    /// Where its links are: filled when it is painted.
    pub fn links(&self) -> LinkBoxes {
        self.links.clone()
    }
}

impl Widget for RichText {
    fn is_leaf(&self) -> bool {
        true
    }

    fn measure(&mut self, cx: &mut MeasureCx, known: Known, avail: Size) -> Size {
        let stale = std::mem::take(&mut self.stale);
        // A box says how wide it is in the style of its span, before the text is shaped
        // around it: the first time, and when the style changed.
        if !self.boxes.is_empty() && (stale || self.paragraph.is_none()) {
            for (span, inline) in &mut self.boxes {
                let shaping = &mut self.spans[*span].shaping;
                let (width, above, below) = inline.measure(cx, &shaping.over(&self.style));
                (shaping.advance, shaping.reach) = (Some(width), (above, below));
            }
            self.paragraph = None;
        }
        let paragraph = match &mut self.paragraph {
            Some(paragraph) => {
                if stale {
                    paragraph.set_style(cx.text, &self.style);
                }
                paragraph
            }
            none => {
                let runs = self.spans.iter().map(|span| (span.end, span.shaping.clone())).collect();
                none.insert(Paragraph::rich(cx.text, self.text.as_str(), self.style.clone(), runs))
            }
        };
        paragraph.wrap(cx.text, known.width.unwrap_or(avail.width))
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        let Some(paragraph) = &self.paragraph else { return };
        let (spans, colors, text) = (&self.spans, &self.colors, self.color);
        let color = |run: usize| colors.get(run).and_then(|of| of.0).unwrap_or(text);
        // A line under each link, two pixels under the baseline, on every line it is on; a
        // line through what is struck, a third of the ascent over it.
        // Each line has its baseline: one with a box higher than its text is higher.
        let (ascent, _) = paragraph.metrics();
        let base = |part: &Rect| {
            let lines = paragraph.lines();
            lines.iter().find(|line| line.y == part.y).map_or(0.0, |line| line.baseline)
        };
        let under = |part: &Rect| base(part).round() + 2.0;
        let through = |part: &Rect| (base(part) - ascent * 0.3).round();
        let (mut start, mut links) = (0, self.links.borrow_mut());
        links.clear();
        for (run, span) in spans.iter().enumerate() {
            if let Some(background) = colors.get(run).and_then(|of| of.1) {
                for part in paragraph.spans(start..span.end, 0.0) {
                    let bounds = Rect::new(cx.rect.x + part.x, cx.rect.y + part.y, part.width, part.height);
                    cx.quad(Quad { bounds, background, ..Quad::default() });
                }
            }
            for part in paragraph.spans(start..span.end, 0.0).filter(|_| span.strike) {
                let bounds =
                    Rect::new(cx.rect.x + part.x, cx.rect.y + part.y + through(&part), part.width, 1.0);
                cx.quad(Quad { bounds, background: color(run), ..Quad::default() });
            }
            let parts = span
                .link
                .iter()
                .flat_map(|link| paragraph.spans(start..span.end, 0.0).map(move |part| (part, link)));
            for (part, link) in parts {
                let bounds =
                    Rect::new(cx.rect.x + part.x, cx.rect.y + part.y + under(&part), part.width, 1.0);
                cx.quad(Quad { bounds, background: color(run), ..Quad::default() });
                links.push((part, link.clone()));
            }
            start = span.end;
        }
        drop(links);
        for k in 0..self.boxes.len() {
            let span = self.boxes[k].0;
            if let Some(part) = paragraph.spans(self.range(span), 0.0).next() {
                let origin = Point::new(cx.rect.x + part.x, cx.rect.y + part.y + base(&part));
                self.boxes[k].1.paint(cx, origin, color(span));
            }
        }
        self.tiles.clear();
        paint_runs(cx, paragraph, cx.rect.origin(), color, &mut self.tiles);
    }

    fn inherit(&mut self, inherited: &Inherited) {
        self.color = inherited.color;
        if self.style != inherited.style {
            self.style.clone_from(&inherited.style);
            self.stale = true;
        }
    }

    fn theme(&mut self, theme: &Theme) -> bool {
        let under = |paint: Option<Paint>| paint.map(|paint| theme.resolve(paint));
        let colors = self.spans.iter().map(|span| (under(span.color), under(span.background))).collect();
        std::mem::replace(&mut self.colors, colors) != self.colors
    }

    fn semantics(&self, says: &mut Semantics) {
        if says.role == Role::None {
            says.role = Role::Text;
        }
        if says.name.is_empty() {
            // A box is said in its place, by what it says it is.
            let mut from = 0;
            for (span, inline) in &self.boxes {
                let at = self.range(*span);
                says.name.extend([&self.text[from..at.start], inline.says()]);
                from = at.end;
            }
            says.name.push_str(&self.text[from..]);
        }
    }
}
