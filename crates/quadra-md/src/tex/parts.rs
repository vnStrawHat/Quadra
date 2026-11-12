//! Fractions, roots, scripts, limits, big operators, accents and lines.

use quadra_prim::Point;

use super::Node;
use super::layout::{Laid, Lay};

/// The style of the lists of a fraction set in each style.
const INNER: [usize; 4] = [1, 2, 3, 3];

impl Lay<'_> {
    pub(super) fn frac(&mut self, over: &[Node], under: &[Node], rule: bool, style: usize) -> Laid {
        let style = style.min(3);
        let (num, den) = (self.list(over, INNER[style]), self.list(under, INNER[style]));
        let (em, display) = (self.em(style), style == 0);
        let (thick, axis) = (if rule { self.rule_width(style) } else { 0.0 }, self.axis(style));
        // How far the baselines of the two lists are from the baseline of the fraction.
        let mut up = em
            * if display {
                0.677
            } else if rule {
                0.394
            } else {
                0.444
            };
        let mut down = em * if display { 0.686 } else { 0.345 };
        if rule {
            let gap = if display { 3.0 * thick } else { thick };
            up = up.max(axis + thick / 2.0 + gap + num.depth);
            down = down.max(den.height - axis + thick / 2.0 + gap);
        } else {
            let gap = self.rule_width(style) * if display { 7.0 } else { 3.0 };
            let short = gap - ((up - num.depth) - (den.height - down));
            if short > 0.0 {
                up += short / 2.0;
                down += short / 2.0;
            }
        }
        let pad = em * 0.12;
        let width = num.width.max(den.width) + 2.0 * pad;
        let mut laid = Laid::sized(width, up + num.height, down + den.depth);
        let (num_x, den_x) = ((width - num.width) / 2.0, (width - den.width) / 2.0);
        laid.put(num, num_x, -up);
        laid.put(den, den_x, down);
        if rule {
            laid.rule(pad * 0.5, -(axis + thick / 2.0), width - pad, thick);
        }
        laid
    }

    pub(super) fn sqrt(&mut self, index: Option<&[Node]>, body: &[Node], style: usize) -> Laid {
        let inner = self.list(body, style);
        let (em, thick) = (self.em(style), self.rule_width(style));
        let clear = if style == 0 { thick + em * 0.11 } else { (thick * 1.25).max(em * 0.08) };
        let top = inner.height.max(em * 0.45) + clear + thick;
        let bottom = inner.depth.max(em * 0.05) + em * 0.04;
        let total = top + bottom;
        let sign = (em * 0.55 + total * 0.05).min(em * 1.1);
        let index = index.map(|index| self.list(index, 3));
        // An index wider than the hook of the sign pushes the sign right.
        let kern = index.as_ref().map_or(0.0, |index| (index.width - sign * 0.45).max(0.0));
        let tick = bottom - (em * 0.5).min(total * 0.45);
        let points = [
            Point::new(kern + em * 0.03, tick + em * 0.07),
            Point::new(kern + em * 0.15, tick),
            Point::new(kern + sign * 0.5, bottom),
            Point::new(kern + sign, -top + thick / 2.0),
            Point::new(kern + sign + inner.width + em * 0.08, -top + thick / 2.0),
        ];
        let mut laid = Laid::sized(points[4].x + em * 0.04, top, bottom);
        laid.stroke(points[..2].to_vec(), thick);
        laid.stroke(points[1..3].to_vec(), thick * 2.0);
        laid.stroke(points[2..4].to_vec(), thick);
        // The bar over what is under the root is a rule: on whole pixels, as the bar of a
        // fraction beside it is.
        laid.rule(points[3].x, -top, points[4].x - points[3].x, thick);
        laid.put(inner, kern + sign + em * 0.04, 0.0);
        if let Some(index) = index {
            let y = bottom - total * 0.6 - index.depth;
            laid.grow(&index, y);
            let x = kern + sign * 0.45 - index.width;
            laid.put(index, x, y);
        }
        laid
    }

    /// A base with what is raised and what is lowered at its side.
    pub(super) fn scripts(
        &mut self,
        base: &Node,
        sup: Option<&[Node]>,
        sub: Option<&[Node]>,
        style: usize,
    ) -> Laid {
        let simple = matches!(base, Node::Atom { .. });
        let base = self.node(base, style);
        let small = Lay::script(style);
        let (em, small_em, x_height) = (self.em(style), self.em(small), self.em(style) * 0.45);
        let (sup, sub) = (sup.map(|sup| self.list(sup, small)), sub.map(|sub| self.list(sub, small)));
        let mut up = if simple { 0.0 } else { base.height - small_em * 0.386 };
        let mut down = if simple { 0.0 } else { base.depth + small_em * 0.05 };
        if let Some(sup) = &sup {
            let least = em * if style == 0 { 0.413 } else { 0.363 };
            up = up.max(least).max(sup.depth + x_height / 4.0);
        }
        match (&sup, &sub) {
            (None, Some(sub)) => down = down.max(em * 0.15).max(sub.height - x_height * 0.8),
            (Some(sup), Some(sub)) => {
                down = down.max(em * 0.247);
                let need = 4.0 * self.rule_width(style) - ((up - sup.depth) - (sub.height - down));
                down += need.max(0.0);
            }
            _ => {}
        }
        let mut laid = Laid::sized(base.width, base.height, base.depth);
        let (at, lean) = (base.width, base.lean);
        laid.put(base, 0.0, 0.0);
        let mut right = at;
        if let Some(sup) = sup {
            laid.grow(&sup, -up);
            right = right.max(at + lean + sup.width);
            laid.put(sup, at + lean, -up);
        }
        if let Some(sub) = sub {
            laid.grow(&sub, down);
            right = right.max(at + sub.width);
            laid.put(sub, at, down);
        }
        laid.width = right + em * 0.05;
        laid
    }

    /// `over` above `base` and `under` below it, each in the middle: the limits of a sum
    /// in display, `\overset`.
    pub(super) fn limits(
        &mut self,
        base: Laid,
        over: Option<&[Node]>,
        under: Option<&[Node]>,
        style: usize,
    ) -> Laid {
        let small = Lay::script(style);
        let gap = self.em(style) * 0.12;
        let (over, under) =
            (over.map(|over| self.list(over, small)), under.map(|under| self.list(under, small)));
        let wide = |part: &Option<Laid>| part.as_ref().map_or(0.0, |part| part.width);
        let width = base.width.max(wide(&over)).max(wide(&under));
        let mut laid = Laid::sized(width, base.height, base.depth);
        let (height, depth, x) = (base.height, base.depth, (width - base.width) / 2.0);
        laid.put(base, x, 0.0);
        if let Some(over) = over {
            let y = -(height + gap + over.depth);
            laid.grow(&over, y);
            let x = (width - over.width) / 2.0;
            laid.put(over, x, y);
        }
        if let Some(under) = under {
            let y = depth + gap + under.height;
            laid.grow(&under, y);
            let x = (width - under.width) / 2.0;
            laid.put(under, x, y);
        }
        laid
    }

    /// A big operator: larger in display, and its middle on the axis.
    pub(super) fn big_op(&mut self, glyph: &str, style: usize) -> Laid {
        let integral = matches!(glyph, "\u{222B}" | "\u{222C}" | "\u{222D}" | "\u{222E}");
        let scale = match (style == 0, integral) {
            (true, true) => 2.0,
            (true, false) => 1.8,
            (false, true) => 1.3,
            (false, false) => 1.1,
        };
        let sign = self.glyph_at(glyph, self.em(style) * scale);
        let shift = (sign.height - sign.depth) / 2.0 - self.axis(style);
        let mut laid =
            Laid::sized(sign.width + self.em(style) * 0.05, sign.height - shift, sign.depth + shift);
        laid.lean = sign.lean;
        laid.put(sign, 0.0, shift);
        laid
    }

    pub(super) fn accent(&mut self, mark: &str, body: &[Node], style: usize) -> Laid {
        let (body, em) = (self.list(body, style), self.em(style));
        // An arrow is as wide as a letter when it is set smaller.
        let mark = self.glyph_at(mark, if mark == "\u{2192}" { em * 0.7 } else { em });
        let y = -(body.height.max(em * 0.45) + em * 0.06) - mark.low.unwrap_or(0.0);
        let width = body.width.max(mark.width);
        let mut laid = Laid::sized(width, -y + mark.height, body.depth);
        let (body_x, mark_x) = ((width - body.width) / 2.0, (width - mark.width) / 2.0 + body.lean / 2.0);
        laid.put(body, body_x, 0.0);
        laid.put(mark, mark_x, y);
        laid
    }

    /// A rule over a list, or under it.
    pub(super) fn line(&mut self, body: &[Node], under: bool, style: usize) -> Laid {
        let (body, thick) = (self.list(body, style), self.rule_width(style));
        let mut laid = Laid::sized(body.width, body.height, body.depth);
        if under {
            laid.rule(0.0, body.depth + 2.0 * thick, body.width, thick);
            laid.depth += 4.0 * thick;
        } else {
            laid.rule(0.0, -(body.height + 3.0 * thick), body.width, thick);
            laid.height += 4.0 * thick;
        }
        laid.put(body, 0.0, 0.0);
        laid
    }
}
