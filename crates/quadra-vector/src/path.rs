//! Paths, and flattening them into polylines.

use quadra_prim::Point;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Step {
    Move(Point),
    Line(Point),
    Quad(Point, Point),
    Cubic(Point, Point, Point),
    Close,
}

/// Lines and curves, in the units of whoever draws them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    steps: Vec<Step>,
    start: Point,
    at: Point,
}

/// A run of straight segments. Closed: the last point joins the first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Polyline {
    pub points: Vec<Point>,
    pub closed: bool,
}

/// A point of the unit circle on the ellipse of an arc.
struct ArcFrame {
    radii: (f64, f64),
    turn: (f64, f64),
    center: (f64, f64),
}

impl ArcFrame {
    fn at(&self, x: f64, y: f64) -> Point {
        let ((rx, ry), (cos, sin), (cx, cy)) = (self.radii, self.turn, self.center);
        Point::new((cos * x * rx - sin * y * ry + cx) as f32, (sin * x * rx + cos * y * ry + cy) as f32)
    }
}

fn angle(ux: f64, uy: f64, vx: f64, vy: f64) -> f64 {
    (ux * vy - uy * vx).atan2(ux * vx + uy * vy)
}

fn quad_at(a: Point, b: Point, c: Point, t: f32) -> Point {
    let u = 1.0 - t;
    let mix = |a: f32, b: f32, c: f32| u * u * a + 2.0 * u * t * b + t * t * c;
    Point::new(mix(a.x, b.x, c.x), mix(a.y, b.y, c.y))
}

fn cubic_at(a: Point, b: Point, c: Point, d: Point, t: f32) -> Point {
    let u = 1.0 - t;
    let mix = |a: f32, b: f32, c: f32, d: f32| {
        u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d
    };
    Point::new(mix(a.x, b.x, c.x, d.x), mix(a.y, b.y, c.y, d.y))
}

impl Path {
    pub fn new() -> Path {
        Path::default()
    }

    pub fn move_to(&mut self, x: f32, y: f32) {
        self.steps.push(Step::Move(Point::new(x, y)));
        (self.start, self.at) = (Point::new(x, y), Point::new(x, y));
    }

    pub fn line_to(&mut self, x: f32, y: f32) {
        self.steps.push(Step::Line(Point::new(x, y)));
        self.at = Point::new(x, y);
    }

    pub fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.steps.push(Step::Quad(Point::new(cx, cy), Point::new(x, y)));
        self.at = Point::new(x, y);
    }

    pub fn cubic_to(&mut self, c1: Point, c2: Point, to: Point) {
        self.steps.push(Step::Cubic(c1, c2, to));
        self.at = to;
    }

    pub fn close(&mut self) {
        self.steps.push(Step::Close);
        self.at = self.start;
    }

    /// Where the path is now.
    pub fn current(&self) -> Point {
        self.at
    }

    /// Appends an elliptical arc as SVG gives it (by its end point) as cubic curves.
    #[allow(clippy::too_many_arguments)] // the seven numbers of an SVG arc
    pub fn arc_to(&mut self, rx: f32, ry: f32, rotation: f32, large: bool, sweep: bool, x: f32, y: f32) {
        let (x1, y1) = (self.at.x, self.at.y);
        if rx == 0.0 || ry == 0.0 || (x1 == x && y1 == y) {
            return self.line_to(x, y);
        }
        let (sin, cos) = (rotation as f64).to_radians().sin_cos();
        let (dx, dy) = ((x1 - x) as f64 / 2.0, (y1 - y) as f64 / 2.0);
        let (x1p, y1p) = (cos * dx + sin * dy, -sin * dx + cos * dy);
        let (mut rx, mut ry) = ((rx as f64).abs(), (ry as f64).abs());
        let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
        if lambda > 1.0 {
            rx *= lambda.sqrt();
            ry *= lambda.sqrt();
        }
        let num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
        let den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
        let mut coef = if den == 0.0 { 0.0 } else { (num / den).max(0.0).sqrt() };
        if large == sweep {
            coef = -coef;
        }
        let (cxp, cyp) = (coef * rx * y1p / ry, -coef * ry * x1p / rx);
        let center =
            (cos * cxp - sin * cyp + (x1 + x) as f64 / 2.0, sin * cxp + cos * cyp + (y1 + y) as f64 / 2.0);
        let from = ((x1p - cxp) / rx, (y1p - cyp) / ry);
        let start = angle(1.0, 0.0, from.0, from.1);
        let mut turn = angle(from.0, from.1, (-x1p - cxp) / rx, (-y1p - cyp) / ry);
        if !sweep && turn > 0.0 {
            turn -= std::f64::consts::TAU;
        } else if sweep && turn < 0.0 {
            turn += std::f64::consts::TAU;
        }
        // A quarter turn a curve at most. A turn that is not a number draws nothing.
        let parts = (turn.abs() / std::f64::consts::FRAC_PI_2).ceil();
        if !parts.is_finite() || parts < 1.0 {
            return self.line_to(x, y);
        }
        let step = turn / parts;
        let k = 4.0 / 3.0 * (step / 4.0).tan();
        let frame = ArcFrame { radii: (rx, ry), turn: (cos, sin), center };
        let mut t = start;
        for _ in 0..parts as usize {
            let ((s1, c1), (s2, c2)) = (t.sin_cos(), (t + step).sin_cos());
            let (a, b) = (frame.at(c1 - k * s1, s1 + k * c1), frame.at(c2 + k * s2, s2 - k * c2));
            self.cubic_to(a, b, frame.at(c2, s2));
            t += step;
        }
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.move_to(x, y);
        self.line_to(x + w, y);
        self.line_to(x + w, y + h);
        self.line_to(x, y + h);
        self.close();
    }

    pub fn rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32) {
        let r = radius.min(w.min(h) / 2.0);
        if r <= 0.0 || !r.is_finite() {
            return self.rect(x, y, w, h);
        }
        self.move_to(x + r, y);
        self.line_to(x + w - r, y);
        self.arc_to(r, r, 0.0, false, true, x + w, y + r);
        self.line_to(x + w, y + h - r);
        self.arc_to(r, r, 0.0, false, true, x + w - r, y + h);
        self.line_to(x + r, y + h);
        self.arc_to(r, r, 0.0, false, true, x, y + h - r);
        self.line_to(x, y + r);
        self.arc_to(r, r, 0.0, false, true, x + r, y);
        self.close();
    }

    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32) {
        self.move_to(cx + rx, cy);
        self.arc_to(rx, ry, 0.0, false, true, cx - rx, cy);
        self.arc_to(rx, ry, 0.0, false, true, cx + rx, cy);
        self.close();
    }

    /// The path as straight segments, each point first scaled and then moved by `offset`
    /// (units to pixels). A curve is cut so that it strays from the true one by about
    /// `tolerance` at most.
    pub fn flatten(&self, scale: f32, offset: Point, tolerance: f32) -> Vec<Polyline> {
        let to = |p: Point| Point::new(p.x * scale + offset.x, p.y * scale + offset.y);
        let (mut out, mut run, mut last) = (Vec::new(), Polyline::default(), Point::ZERO);
        let keep = |run: &mut Polyline, next: Point, out: &mut Vec<Polyline>| {
            let done = std::mem::replace(run, Polyline { points: vec![next], closed: false });
            if done.points.len() > 1 {
                out.push(done);
            }
        };
        for step in &self.steps {
            match *step {
                Step::Move(p) => {
                    last = to(p);
                    keep(&mut run, last, &mut out);
                }
                Step::Line(p) => {
                    last = to(p);
                    run.points.push(last);
                }
                Step::Quad(..) | Step::Cubic(..) => {
                    let (from, ctrl) = (
                        last,
                        match *step {
                            Step::Quad(a, b) => [to(a), to(b), to(b)],
                            Step::Cubic(a, b, c) => [to(a), to(b), to(c)],
                            _ => unreachable!("only curves come here"),
                        },
                    );
                    let far = |q: &Point| (q.x - from.x).abs().max((q.y - from.y).abs());
                    let span = ctrl.iter().map(far).fold(0.0, f32::max);
                    // Between 2 and 64 pieces; a span that is not a number gets the fewest.
                    let pieces = ((span / tolerance).sqrt() * 2.0).clamp(2.0, 64.0);
                    let pieces = if pieces.is_finite() { pieces as usize } else { 2 };
                    for i in 1..=pieces {
                        let t = i as f32 / pieces as f32;
                        run.points.push(match *step {
                            Step::Quad(..) => quad_at(from, ctrl[0], ctrl[1], t),
                            _ => cubic_at(from, ctrl[0], ctrl[1], ctrl[2], t),
                        });
                    }
                    last = ctrl[2];
                }
                Step::Close => {
                    run.closed = true;
                    last = run.points.first().copied().unwrap_or(last);
                    keep(&mut run, last, &mut out);
                }
            }
        }
        if run.points.len() > 1 {
            out.push(run);
        }
        out
    }

    /// A number for the path's shape: what a cache of rasterized paths goes by.
    pub fn hash(&self) -> u64 {
        const PRIME: u64 = 1099511628211;
        let mix = |h: u64, v: u64| (h ^ v).wrapping_mul(PRIME);
        let point = |h: u64, p: Point| mix(mix(h, p.x.to_bits() as u64), p.y.to_bits() as u64);
        self.steps.iter().fold(1469598103934665603, |h, step| match *step {
            Step::Move(a) => point(mix(h, 0), a),
            Step::Line(a) => point(mix(h, 1), a),
            Step::Quad(a, b) => point(point(mix(h, 2), a), b),
            Step::Cubic(a, b, c) => point(point(point(mix(h, 3), a), b), c),
            Step::Close => mix(h, 4),
        })
    }
}
