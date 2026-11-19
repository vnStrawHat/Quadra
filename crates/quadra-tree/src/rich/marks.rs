//! Ranges of a rich text marked apart from its selection (specs/05, "Rules"): where a find
//! over a document's texts matches, the current one in a stronger fill.

use std::ops::Range;

use quadra_prim::{Color, Rect};
use quadra_render::Quad;
use quadra_text::Paragraph;

use super::RichText;
use crate::widget::PaintCx;

#[derive(Default)]
pub(super) struct Marks {
    ranges: Vec<Range<usize>>,
    current: Option<usize>,
    fills: (Color, Color),
}

impl Marks {
    /// Paints the marks under the selection and the words.
    pub(super) fn paint(&self, cx: &mut PaintCx, paragraph: &Paragraph) {
        for (k, range) in self.ranges.iter().enumerate() {
            let background = if self.current == Some(k) { self.fills.1 } else { self.fills.0 };
            for part in paragraph.spans(range.clone(), 0.0) {
                let bounds = Rect::new(cx.rect.x + part.x, cx.rect.y + part.y, part.width, part.height);
                cx.quad(Quad { bounds, background, ..Quad::default() });
            }
        }
    }
}

impl RichText {
    /// Its text, a box's span one U+FFFC.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Marks ranges of its text in `fill`, the `current`th of them in `chosen`; none for no
    /// marks. True when that is another.
    pub fn mark(
        &mut self,
        ranges: Vec<Range<usize>>,
        current: Option<usize>,
        fill: Color,
        chosen: Color,
    ) -> bool {
        let marks = Marks { ranges, current, fills: (fill, chosen) };
        let same = marks.ranges == self.marks.ranges && marks.current == self.marks.current;
        let same = same && (marks.ranges.is_empty() || marks.fills == self.marks.fills);
        self.marks = marks;
        !same
    }

    /// Where the first part of a range of its text is from the node's corner, once shaped.
    pub fn rect_of(&self, range: Range<usize>) -> Option<Rect> {
        self.paragraph.as_ref()?.spans(range, 0.0).next()
    }
}
