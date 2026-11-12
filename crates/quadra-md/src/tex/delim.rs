//! Delimiters that grow with what is between them. A little taller than its glyph, one is
//! the glyph set larger; taller than that, it is drawn as strokes.

use quadra_prim::Point;

use super::layout::{Laid, Lay};

/// Twelve steps of a curve from `a` to `b`, bent towards `c`.
fn curve(points: &mut Vec<Point>, a: Point, c: Point, b: Point) {
    for step in 0..=12 {
        let t = step as f32 / 12.0;
        let s = 1.0 - t;
        let mix = |a: f32, c: f32, b: f32| s * s * a + 2.0 * s * t * c + t * t * b;
        points.push(Point::new(mix(a.x, c.x, b.x), mix(a.y, c.y, b.y)));
    }
}

impl Lay<'_> {
    /// `inner` between delimiters grown to it: they cover nine tenths of how far it
    /// reaches from the axis, and all of it but a quarter of an em at the least.
    pub(super) fn wrap(&mut self, inner: Laid, left: &str, right: &str, style: usize) -> Laid {
        let axis = self.axis(style);
        let reach = (inner.height - axis).max(inner.depth + axis);
        let half = (reach * 0.901).max(reach - self.em(style) * 0.25);
        let (left, right) = (self.delimiter(left, half, style), self.delimiter(right, half, style));
        let mut laid = Laid::default();
        for part in [left, inner, right] {
            laid.grow(&part, 0.0);
            let x = laid.width;
            laid.width += part.width;
            laid.put(part, x, 0.0);
        }
        laid
    }

    /// A delimiter that covers `half` above the axis and as much below it; none is a
    /// little space.
    pub(super) fn delimiter(&mut self, delim: &str, half: f32, style: usize) -> Laid {
        let em = self.em(style);
        if delim.is_empty() {
            return Laid::sized(em * 0.12, 0.0, 0.0);
        }
        let axis = self.axis(style);
        let mut glyph = self.glyph(delim, style);
        let ink = glyph.height + glyph.depth;
        let scale = if ink > 0.0 { 2.0 * half / ink } else { 1.0 };
        if scale > 1.6 {
            return self.drawn(delim, half, style);
        }
        if scale > 1.05 {
            glyph = self.glyph_at(delim, em * scale);
        }
        let shift = (glyph.height - glyph.depth) / 2.0 - axis;
        let mut laid = Laid::sized(glyph.width, glyph.height - shift, glyph.depth + shift);
        laid.put(glyph, 0.0, shift);
        laid
    }

    /// A tall delimiter, as strokes: its middle on the axis.
    fn drawn(&mut self, delim: &str, half: f32, style: usize) -> Laid {
        let (em, axis) = (self.em(style), self.axis(style));
        let thick = em * 0.08;
        let (width, side) = ((em * 0.32 + half * 0.08).min(em * 0.6), em * 0.08);
        let (top, mid, bottom) = (-axis - half, -axis, -axis + half);
        let (x0, x1) = (side, side + width);
        let at = Point::new;
        let mut laid = Laid::sized(width + 2.0 * side, axis + half, half - axis);
        let mut points = Vec::new();
        match delim {
            "(" | ")" => curve(&mut points, at(x1, top), at(x0 - width * 0.9, mid), at(x1, bottom)),
            "[" | "]" => {
                points = vec![at(x1, top), at(x0 + thick, top), at(x0 + thick, bottom), at(x1, bottom)]
            }
            "\u{230A}" | "\u{230B}" => {
                points = vec![at(x0 + thick, top), at(x0 + thick, bottom), at(x1, bottom)]
            }
            "\u{2308}" | "\u{2309}" => {
                points = vec![at(x1, top), at(x0 + thick, top), at(x0 + thick, bottom)]
            }
            "\u{27E8}" | "\u{27E9}" => points = vec![at(x1, top), at(x0, mid), at(x1, bottom)],
            "{" | "}" => {
                let middle = (x0 + x1) / 2.0;
                curve(&mut points, at(x1, top), at(middle, top), at(middle, top + half * 0.3));
                curve(&mut points, at(middle, mid - half * 0.25), at(middle, mid), at(x0, mid));
                curve(&mut points, at(x0, mid), at(middle, mid), at(middle, mid + half * 0.25));
                curve(&mut points, at(middle, bottom - half * 0.3), at(middle, bottom), at(x1, bottom));
            }
            "\u{2016}" => {
                laid.stroke(vec![at(x0 + width * 0.3, top), at(x0 + width * 0.3, bottom)], thick);
                points = vec![at(x0 + width * 0.7, top), at(x0 + width * 0.7, bottom)];
            }
            "/" => points = vec![at(x1, top), at(x0, bottom)],
            // A bar, and whatever else: a bar.
            _ => points = vec![at((x0 + x1) / 2.0, top), at((x0 + x1) / 2.0, bottom)],
        }
        if matches!(delim, ")" | "]" | "}" | "\u{27E9}" | "\u{230B}" | "\u{2309}") {
            points.iter_mut().for_each(|point| point.x = x0 + x1 - point.x);
        }
        laid.stroke(points, thick);
        laid
    }
}
