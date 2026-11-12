//! The rich text widget: one paragraph of spans, each with what it has otherwise than the
//! text values that hold at the node (specs/05, "Rules"; specs/08, "Paragraphs").

use std::cell::RefCell;
use std::rc::Rc;

use quadra_a11y::{Role, Semantics};
use quadra_layout::Known;
use quadra_prim::{Color, Rect, Size};
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
}

impl RichText {
    pub fn new(text: impl Into<String>, spans: Vec<RichSpan>) -> RichText {
        let (text, style, color) = (text.into(), TextStyle::default(), Color::BLACK);
        let (links, tiles) = (LinkBoxes::default(), Vec::new());
        let colors = Vec::new(); // the tree gives the theme when the node is added
        RichText { text, spans, colors, style, color, paragraph: None, stale: false, links, tiles }
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
        let paragraph = match &mut self.paragraph {
            Some(paragraph) => {
                if std::mem::take(&mut self.stale) {
                    paragraph.set_style(cx.text, &self.style);
                }
                paragraph
            }
            none => {
                self.stale = false;
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
        let (ascent, descent) = paragraph.metrics();
        let base = (self.style.line_px() - ascent - descent) / 2.0 + ascent;
        let (under, through) = (base.round() + 2.0, (base - ascent * 0.3).round());
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
                let bounds = Rect::new(cx.rect.x + part.x, cx.rect.y + part.y + through, part.width, 1.0);
                cx.quad(Quad { bounds, background: color(run), ..Quad::default() });
            }
            let parts = span
                .link
                .iter()
                .flat_map(|link| paragraph.spans(start..span.end, 0.0).map(move |part| (part, link)));
            for (part, link) in parts {
                let bounds = Rect::new(cx.rect.x + part.x, cx.rect.y + part.y + under, part.width, 1.0);
                cx.quad(Quad { bounds, background: color(run), ..Quad::default() });
                links.push((part, link.clone()));
            }
            start = span.end;
        }
        drop(links);
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
            says.name.clone_from(&self.text);
        }
    }
}
