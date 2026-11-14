//! The text field as a widget: how it is measured and painted, and what it says of itself.

use std::ops::Range;

use quadra_a11y::{Role, Semantics};
use quadra_layout::Known;
use quadra_prim::{Color, Point, Rect, Size};
use quadra_render::Quad;

use super::TextField;
use crate::inherit::Inherited;
use crate::text::paint_glyphs;
use crate::widget::{MeasureCx, PaintCx, Widget};

impl Widget for TextField {
    fn is_leaf(&self) -> bool {
        true
    }

    /// As wide as the space it is given; only where the space has no end is it as wide as
    /// its text. One line high and the text scrolls inside it, or as high as the lines the
    /// text takes at that width.
    fn measure(&mut self, cx: &mut MeasureCx, known: Known, avail: Size) -> Size {
        self.width = known.width.unwrap_or(avail.width);
        self.sync(cx.text);
        let (text, line_px) =
            (self.paragraph.as_ref().map_or(Size::ZERO, |p| p.size()), self.style.line_px());
        let fits = self.fit && !self.multiline;
        let width = if self.width.is_finite() && !fits { self.width } else { text.width + 2.0 }; // + the caret
        Size::new(width, if self.multiline { text.height.max(line_px) } else { line_px })
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        let (_, caret_index, composed, selected) = self.shown();
        let [color, placeholder, selection, caret_color] = self.colors;
        let Some(paragraph) = &self.paragraph else { return };
        let (rect, line_px) = (cx.rect, self.style.line_px());
        let (line, caret) = paragraph.place(caret_index);
        // One line fills the field, however tall that is, and scrolls sideways: the caret
        // stays in view, and the text never leaves a gap it could fill.
        let (top, tall) = if self.multiline { (line as f32 * line_px, line_px) } else { (0.0, rect.height) };
        let room = (rect.width - 1.0).max(0.0);
        self.scroll_x = self.scroll_x.clamp((caret - room).max(0.0), caret);
        self.scroll_x = self.scroll_x.min((paragraph.size().width - room).max(0.0));
        if self.multiline {
            self.scroll_x = 0.0;
        }
        let left = rect.x - self.scroll_x;
        let (outer, inner) = (cx.clip, cx.clip.then(cx.device(rect), 0.0));
        cx.clip = inner;

        // A range that goes on past the end of a line shows a little of the line break.
        let boxes = |range: Range<usize>| paragraph.spans(range, self.style.size * 0.3);
        if !composed.is_empty() {
            // What is being composed is underlined, as input methods show it.
            for span in boxes(composed) {
                let bounds = Rect::new(left + span.x, rect.y + span.y + tall - 1.0, span.width, 1.0);
                cx.quad(Quad { bounds, background: color, ..Quad::default() });
            }
        } else {
            for span in boxes(selected) {
                let bounds = Rect::new(left + span.x, rect.y + span.y, span.width, tall);
                cx.quad(Quad { bounds, background: selection, ..Quad::default() });
            }
        }
        self.tiles.clear();
        // An empty field shows its placeholder, until something is typed or composed.
        match self.hint.as_ref().filter(|_| paragraph.text().is_empty()) {
            Some(hint) => paint_glyphs(cx, hint, Point::new(left, rect.y), placeholder, &mut self.tiles),
            None => paint_glyphs(cx, paragraph, Point::new(left, rect.y), color, &mut self.tiles),
        }
        if self.focused {
            // One device pixel wide at least, on a whole pixel: a caret is never blurred.
            let x = ((left + caret) * cx.scale).round() / cx.scale;
            let width = cx.scale.round().max(1.0) / cx.scale;
            let bounds = cx.device(Rect::new(x, rect.y + top, width, tall));
            // Always one instance, clear while the blink hides it: a blink overwrites it in
            // place and moves nothing else of the layer.
            let background = if self.caret_on { caret_color } else { Color::TRANSPARENT };
            cx.push(Quad { bounds, background, ..Quad::default() }.instance(&cx.clip));
        }
        cx.clip = outer;
        self.caret_at = Some(Rect::new(left + caret - rect.x, top, 1.0, tall));
    }

    fn inherit(&mut self, inherited: &Inherited) {
        self.colors = [inherited.color, inherited.placeholder, inherited.selection, inherited.caret];
        self.style.clone_from(&inherited.style);
    }

    fn caret(&self) -> Option<Rect> {
        self.caret_at
    }

    fn semantics(&self, says: &mut Semantics) {
        if says.role == Role::None {
            says.role = Role::TextInput;
        }
        // Of a password, the dots. The selection in characters: where it was begun, and the
        // caret.
        let (shown, caret, _, selected) = self.shown();
        let anchor = if caret == selected.start { selected.end } else { selected.start };
        let at = |index: usize| super::characters(&shown[..index.min(shown.len())]);
        says.selection = Some((at(anchor), at(caret)));
        says.value = Some(shown.into_owned());
        if says.name.is_empty() {
            says.name.clone_from(&self.placeholder); // what it asks for, when nothing names it
        }
    }
}
