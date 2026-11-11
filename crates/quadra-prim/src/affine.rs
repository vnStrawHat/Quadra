//! 2D affine transforms: what a layer applies to everything in it (specs/07-render.md, Layers).

use crate::{Point, Rect};

/// Maps `(x, y)` to `(a*x + c*y + tx, b*x + d*y + ty)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub tx: f32,
    pub ty: f32,
}

impl Default for Affine {
    fn default() -> Affine {
        Affine::IDENTITY
    }
}

impl Affine {
    pub const IDENTITY: Affine = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    pub const fn translate(dx: f32, dy: f32) -> Affine {
        Affine {
            tx: dx,
            ty: dy,
            ..Affine::IDENTITY
        }
    }

    /// Scales about the origin.
    pub const fn scale(s: f32) -> Affine {
        Affine {
            a: s,
            d: s,
            ..Affine::IDENTITY
        }
    }

    /// Rotates by `degrees`, clockwise on screen (y grows downwards), about `pivot`.
    pub fn rotate_about(degrees: f32, pivot: Point) -> Affine {
        let (sin, cos) = degrees.to_radians().sin_cos();
        Affine {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            tx: pivot.x - (cos * pivot.x - sin * pivot.y),
            ty: pivot.y - (sin * pivot.x + cos * pivot.y),
        }
    }

    pub fn apply(self, p: Point) -> Point {
        Point::new(
            self.a * p.x + self.c * p.y + self.tx,
            self.b * p.x + self.d * p.y + self.ty,
        )
    }

    /// `self` applied after `inner`: `p -> self(inner(p))`.
    pub fn then(self, inner: Affine) -> Affine {
        let t = self.apply(Point::new(inner.tx, inner.ty));
        Affine {
            a: self.a * inner.a + self.c * inner.b,
            b: self.b * inner.a + self.d * inner.b,
            c: self.a * inner.c + self.c * inner.d,
            d: self.b * inner.c + self.d * inner.d,
            tx: t.x,
            ty: t.y,
        }
    }

    /// `None` for a transform that flattens the plane (a scale of zero).
    pub fn inverse(self) -> Option<Affine> {
        let det = self.a * self.d - self.b * self.c;
        if det == 0.0 || !det.is_finite() {
            return None;
        }
        let (a, b, c, d) = (self.d / det, -self.b / det, -self.c / det, self.a / det);
        Some(Affine {
            a,
            b,
            c,
            d,
            tx: -(a * self.tx + c * self.ty),
            ty: -(b * self.tx + d * self.ty),
        })
    }

    pub fn is_identity(self) -> bool {
        self == Affine::IDENTITY
    }

    /// Moves without scaling or turning: scrolling. Glyphs keep their pixels under it.
    pub fn is_translation(self) -> bool {
        self.a == 1.0 && self.b == 0.0 && self.c == 0.0 && self.d == 1.0
    }

    /// The axis-aligned box around the mapped rectangle.
    pub fn bbox(self, r: Rect) -> Rect {
        if self.is_translation() {
            return r.translate(self.tx, self.ty);
        }
        let corners = [
            self.apply(Point::new(r.x, r.y)),
            self.apply(Point::new(r.right(), r.y)),
            self.apply(Point::new(r.right(), r.bottom())),
            self.apply(Point::new(r.x, r.bottom())),
        ];
        let (mut x0, mut y0, mut x1, mut y1) =
            (corners[0].x, corners[0].y, corners[0].x, corners[0].y);
        for p in &corners[1..] {
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
            x1 = x1.max(p.x);
            y1 = y1.max(p.y);
        }
        Rect::new(x0, y0, x1 - x0, y1 - y0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Point, b: Point) -> bool {
        (a.x - b.x).abs() < 1e-4 && (a.y - b.y).abs() < 1e-4
    }

    #[test]
    fn a_quarter_turn_is_clockwise_about_its_pivot() {
        let turn = Affine::rotate_about(90.0, Point::new(10.0, 10.0));
        assert!(close(
            turn.apply(Point::new(10.0, 10.0)),
            Point::new(10.0, 10.0)
        ));
        // The point to the right of the pivot goes below it.
        assert!(close(
            turn.apply(Point::new(15.0, 10.0)),
            Point::new(10.0, 15.0)
        ));
    }

    #[test]
    fn then_composes_and_inverse_undoes() {
        let t = Affine::translate(5.0, -3.0).then(Affine::scale(2.0));
        assert_eq!(t.apply(Point::new(1.0, 1.0)), Point::new(7.0, -1.0));
        let p = Point::new(3.5, -8.0);
        let m = Affine::rotate_about(33.0, Point::new(4.0, 9.0)).then(t);
        assert!(close(m.inverse().unwrap().apply(m.apply(p)), p));
        assert_eq!(Affine::scale(0.0).inverse(), None);
        assert!(Affine::default().is_identity() && Affine::translate(1.0, 0.0).is_translation());
        assert!(!Affine::scale(2.0).is_translation());
    }

    #[test]
    fn bbox_surrounds_the_mapped_rect() {
        let r = Rect::new(0.0, 0.0, 10.0, 4.0);
        assert_eq!(
            Affine::translate(3.0, 2.0).bbox(r),
            Rect::new(3.0, 2.0, 10.0, 4.0)
        );
        let b = Affine::rotate_about(90.0, Point::ZERO).bbox(r);
        assert!(close(b.origin(), Point::new(-4.0, 0.0)));
        assert!((b.width - 4.0).abs() < 1e-4 && (b.height - 10.0).abs() < 1e-4);
    }
}
