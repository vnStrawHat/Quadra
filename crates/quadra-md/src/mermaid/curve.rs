//! The geometry the kinds of diagram share: a smooth curve through points, where a line
//! from the middle of a node leaves its outline, a line made shorter for its head, and
//! dashes.

use quadra_prim::{Point, Rect};
use quadra_vector::Path;

use super::draw::Head;

fn between(a: Point, b: Point, t: f32) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

pub fn dist(a: Point, b: Point) -> f32 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// The curve from the first point to the last that the points between pull on (a uniform
/// B-spline), as a line through points.
pub fn basis(points: &[Point]) -> Vec<Point> {
    let [first, .., last] = points else { return points.to_vec() };
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut path = Path::new();
    path.move_to(first.x, first.y);
    let start = between(points[0], points[1], 1.0 / 6.0);
    path.line_to(start.x, start.y);
    // Each three points in a row make a piece; the last point counts twice.
    let pieces = points.windows(3).map(|w| (w[0], w[1], w[2]));
    for (a, b, c) in pieces.chain([(points[points.len() - 2], *last, *last)]) {
        let to = Point::new((a.x + 4.0 * b.x + c.x) / 6.0, (a.y + 4.0 * b.y + c.y) / 6.0);
        path.cubic_to(between(a, b, 1.0 / 3.0), between(a, b, 2.0 / 3.0), to);
    }
    path.line_to(last.x, last.y);
    path.flatten(1.0, Point::ZERO, 0.2).into_iter().next().map_or_else(|| points.to_vec(), |line| line.points)
}

/// What the outline of a node is cut by.
#[derive(Clone, Copy, PartialEq)]
pub enum Outline {
    Box,
    Diamond,
    Circle,
}

/// Where the line from the middle of `r` towards `to` leaves the outline.
pub fn clip(r: Rect, outline: Outline, to: Point) -> Point {
    let c = r.center();
    let (dx, dy) = (to.x - c.x, to.y - c.y);
    if (dx == 0.0 && dy == 0.0) || r.width <= 0.0 || r.height <= 0.0 {
        return c;
    }
    let (hw, hh) = (r.width / 2.0, r.height / 2.0);
    let t = match outline {
        Outline::Diamond => 1.0 / (dx.abs() / hw + dy.abs() / hh),
        Outline::Circle => hw / dx.hypot(dy),
        // A division by nothing is past any end: the other side cuts.
        Outline::Box => (hw / dx.abs()).min(hh / dy.abs()),
    };
    Point::new(c.x + dx * t, c.y + dy * t)
}

/// The line without its last `length` pixels.
pub fn cut_end(points: &[Point], length: f32) -> Vec<Point> {
    if length <= 0.0 || points.len() < 2 {
        return points.to_vec();
    }
    let mut left = length;
    for i in (1..points.len()).rev() {
        let piece = dist(points[i - 1], points[i]);
        if piece >= left {
            let mut out = points[..i].to_vec();
            out.push(between(points[i - 1], points[i], (piece - left) / piece.max(1e-6)));
            return out;
        }
        left -= piece;
    }
    vec![points[0], points[0]]
}

/// How far before its tip a line stops for its head.
pub fn head_len(head: Head, width: f32) -> f32 {
    match head {
        Head::Arrow => 7.0 + 2.0 * width,
        Head::Circle => 8.0,
        Head::Cross => 5.0,
        Head::None | Head::Open => 0.0,
    }
}

/// The line in dashes `on` long with `off` between them.
pub fn dashes(points: &[Point], on: f32, off: f32) -> Vec<Vec<Point>> {
    let (mut out, mut dash) = (Vec::new(), Vec::new());
    let (mut drawing, mut left) = (true, on);
    for pair in points.windows(2) {
        let (mut a, b) = (pair[0], pair[1]);
        let mut piece = dist(a, b);
        if drawing && dash.is_empty() {
            dash.push(a);
        }
        while piece > left {
            a = between(a, b, left / piece);
            if drawing {
                dash.push(a);
                out.push(std::mem::take(&mut dash));
            } else {
                dash = vec![a];
            }
            piece -= left;
            drawing = !drawing;
            left = if drawing { on } else { off };
        }
        left -= piece;
        if drawing {
            dash.push(b);
        }
    }
    if dash.len() > 1 {
        out.push(dash);
    }
    out
}
