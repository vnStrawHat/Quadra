//! Rasterizing to coverage on the CPU: fills by accumulating signed areas (exact edges),
//! strokes by the distance to their segments (round caps and joins come with it).

use quadra_prim::Point;

use crate::path::Polyline;

/// How much of each pixel is covered, from 0 to 1.
pub struct Coverage {
    pub width: usize,
    pub height: usize,
    /// A row is `width + 1` values: one to spare at the end.
    data: Vec<f32>,
}

fn finite(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

/// Adds what one edge contributes to the signed areas of the rows it crosses.
fn accumulate(acc: &mut [f32], stride: usize, height: usize, p0: Point, p1: Point) {
    // An edge with a coordinate that is no number adds nothing.
    if p0.y == p1.y || !finite(p0) || !finite(p1) {
        return;
    }
    let dir = if p0.y < p1.y { 1.0 } else { -1.0 };
    let (a, b) = if p0.y < p1.y { (p0, p1) } else { (p1, p0) };
    let dxdy = (b.x - a.x) / (b.y - a.y);
    let mut x = if a.y < 0.0 { a.x - a.y * dxdy } else { a.x };
    let mut add = |i: isize, v: f32| {
        if let Some(cell) = usize::try_from(i).ok().and_then(|i| acc.get_mut(i)) {
            *cell += v;
        }
    };
    // Clamped as floats: a row range of a number past the bitmap would never end.
    let rows = a.y.floor().clamp(0.0, height as f32) as usize..b.y.ceil().clamp(0.0, height as f32) as usize;
    for y in rows {
        let line = (y * stride) as isize;
        let dy = ((y + 1) as f32).min(b.y) - (y as f32).max(a.y);
        let next = x + dxdy * dy;
        let d = dy * dir;
        let (x0, x1) = if x < next { (x, next) } else { (next, x) };
        let (x0floor, x1ceil) = (x0.floor(), x1.ceil());
        let (x0i, x1i) = (x0floor as isize, x1ceil as isize);
        if x1i <= x0i + 1 {
            let mid = 0.5 * (x + next) - x0floor;
            add(line + x0i, d - d * mid);
            add(line + x0i + 1, d * mid);
        } else {
            let s = 1.0 / (x1 - x0);
            let x0f = x0 - x0floor;
            let a0 = 0.5 * s * (1.0 - x0f) * (1.0 - x0f);
            let x1f = x1 - x1ceil + 1.0;
            let am = 0.5 * s * x1f * x1f;
            add(line + x0i, d * a0);
            if x1i == x0i + 2 {
                add(line + x0i + 1, d * (1.0 - a0 - am));
            } else {
                let a1 = s * (1.5 - x0f);
                add(line + x0i + 1, d * (a1 - a0));
                for xi in x0i + 2..x1i - 1 {
                    add(line + xi, d * s);
                }
                let a2 = a1 + (x1i - x0i - 3) as f32 * s;
                add(line + x1i - 1, d * (1.0 - a2 - am));
            }
            add(line + x1i, d * am);
        }
        x = next;
    }
}

impl Coverage {
    pub fn new(width: usize, height: usize) -> Coverage {
        Coverage { width, height, data: vec![0.0; (width + 1) * height] }
    }

    /// The coverage of a pixel.
    pub fn at(&self, x: usize, y: usize) -> f32 {
        self.data[y * (self.width + 1) + x]
    }

    /// Covers the inside of closed polylines.
    pub fn fill(&mut self, lines: &[Polyline]) {
        let (stride, width) = (self.width + 2, self.width as f32);
        let mut acc = vec![0.0f32; stride * (self.height + 1)];
        for line in lines {
            let count = line.points.len();
            for i in 0..count {
                let (a, b) = (line.points[i], line.points[(i + 1) % count]);
                // An edge is cut where it crosses a side of the bitmap, and what is off it
                // sideways is brought to that side: the area inside stays exact, and what
                // was left of it lands in the first column.
                let mut cuts = [0.0f32, 1.0, 1.0, 1.0];
                for (k, side) in [0.0, width].into_iter().enumerate() {
                    let t = (side - a.x) / (b.x - a.x);
                    if t > 0.0 && t < 1.0 {
                        cuts[k + 1] = t;
                    }
                }
                cuts[1..3].sort_by(f32::total_cmp);
                let at = |t: f32| {
                    let p = Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
                    Point::new(p.x.clamp(0.0, width), p.y)
                };
                for piece in cuts.windows(2).filter(|piece| piece[0] < piece[1]) {
                    accumulate(&mut acc, stride, self.height, at(piece[0]), at(piece[1]));
                }
            }
        }
        for y in 0..self.height {
            let mut sum = 0.0f32;
            for x in 0..self.width {
                sum += acc[y * stride + x];
                let cell = &mut self.data[y * (self.width + 1) + x];
                *cell = cell.max(sum.abs().min(1.0)).min(1.0);
            }
        }
    }

    /// Covers polylines drawn `width` pixels thick, with round caps and joins.
    pub fn stroke(&mut self, lines: &[Polyline], width: f32) {
        let half = width / 2.0;
        for line in lines {
            let count = line.points.len();
            let segments = if line.closed { count } else { count.saturating_sub(1) };
            for i in 0..segments {
                self.capsule(line.points[i], line.points[(i + 1) % count], half);
            }
            if count == 1 {
                self.capsule(line.points[0], line.points[0], half);
            }
        }
    }

    fn capsule(&mut self, a: Point, b: Point, half: f32) {
        if !finite(a) || !finite(b) || !half.is_finite() || self.width == 0 || self.height == 0 {
            return;
        }
        // The box is cut to the bitmap as floats: far off, it is no box at all.
        let (w, h) = ((self.width - 1) as f32, (self.height - 1) as f32);
        let x0 = (a.x.min(b.x) - half - 1.0).clamp(0.0, w) as usize;
        let y0 = (a.y.min(b.y) - half - 1.0).clamp(0.0, h) as usize;
        let x1 = (a.x.max(b.x) + half + 1.0).clamp(-1.0, w);
        let y1 = (a.y.max(b.y) + half + 1.0).clamp(-1.0, h);
        if x1 < 0.0 || y1 < 0.0 {
            return;
        }
        let (abx, aby) = (b.x - a.x, b.y - a.y);
        let len2 = abx * abx + aby * aby;
        for y in y0..=y1 as usize {
            for x in x0..=x1 as usize {
                let (px, py) = (x as f32 + 0.5 - a.x, y as f32 + 0.5 - a.y);
                let t = if len2 > 0.0 { ((px * abx + py * aby) / len2).clamp(0.0, 1.0) } else { 0.0 };
                let (dx, dy) = (px - abx * t, py - aby * t);
                let covered = (half + 0.5 - dx.hypot(dy)).clamp(0.0, 1.0);
                let cell = &mut self.data[y * (self.width + 1) + x];
                *cell = cell.max(covered);
            }
        }
    }

    /// The coverage as a byte a pixel, row by row.
    pub fn to_bytes(&self) -> Vec<u8> {
        let rows = (0..self.height).flat_map(|y| (0..self.width).map(move |x| (x, y)));
        rows.map(|(x, y)| (self.at(x, y).clamp(0.0, 1.0) * 255.0 + 0.5) as u8).collect()
    }
}
