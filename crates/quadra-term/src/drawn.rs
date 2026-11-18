//! Characters drawn, not the font's (specs/18, "Drawn characters"): box drawing, block
//! elements and the separators of patched fonts, each a coverage mask of its cells' box in
//! whole device pixels, the mirror of its mirrored character about both middles.

use oneterm_vt::SnapshotContent;
use quadra_prim::{Color, Corners, Rect};
use quadra_render::{Kind, Sprite};
use quadra_tree::PaintCx;

use crate::widget::TerminalWidget;

#[cfg(test)]
mod tests;

/// Whether `c` is drawn here.
pub(crate) fn drawn(c: char) -> bool {
    matches!(c as u32, 0x2500..=0x259F | 0xE0B0..=0xE0B3)
}

/// Paints a cell's character over its cells' box `b` (logical pixels) in `fg`, as a glyph is,
/// if it is drawn here.
pub(crate) fn paint(
    w: &mut TerminalWidget,
    cx: &mut PaintCx,
    content: SnapshotContent,
    b: Rect,
    fg: Color,
) -> bool {
    let SnapshotContent::Scalar(c) = content else { return false };
    if !drawn(c) {
        return false;
    }
    let s = cx.scale;
    let (x0, y0) = ((b.x * s).round(), (b.y * s).round());
    let (wide, tall) = (((b.x + b.width) * s).round() - x0, ((b.y + b.height) * s).round() - y0);
    let (wide, tall) = (wide.max(1.0) as u32, tall.max(1.0) as u32);
    let shape = 0x5A << 56 | u64::from(tall) << 37 | u64::from(wide) << 21 | c as u64;
    let tile = cx.fonts.mask(shape, wide, tall, || mask(c, wide, tall));
    let Some(tile) = tile.filter(|it| it.rect.w > 0) else { return true };
    let sprite = Sprite {
        bounds: Rect::new(x0, y0, tile.rect.w as f32, tile.rect.h as f32),
        uv: tile.uv,
        slice: tile.rect.slice,
        color: fg,
        radii: Corners::default(),
        glyph: true,
    };
    cx.push(sprite.instance(Kind::Mono, &cx.clip));
    w.tiles.push(tile);
    true
}

/// The arms of U+2500 to U+257F, up, right, down, left: 0 none, 1 light, 2 heavy, 3 double;
/// `0000` for those drawn otherwise (dashes, arcs, diagonals).
const ARMS: &str = concat!(
    "0101020210102020",
    "00000000000000000000000000000000",
    "0110021001200220001100120021002211001200210022001001100220012002",
    "1110121021101120212022101220222010111012201110212021201210222022",
    "0111011202110212012101220221022211011102120112022101210222012202",
    "1111111212111212211111212121211222111122122122121222212222212222",
    "0000000000000000",
    "0303303003100130033000130031003313003100330010033001300313103130",
    "3330101330313033031301310333130331013303131331313333",
    "0000000000000000000000000000",
    "000110000100001000022000020000200201102001022010",
);

/// The coverage of `c` in a box of `w` x `h` device pixels, a byte a pixel, rows from the top.
pub(crate) fn mask(c: char, w: u32, h: u32) -> Vec<u8> {
    let mut m = Mask { w: w as usize, h: h as usize, px: vec![0.0; (w * h) as usize] };
    let n = c as u32;
    let (wf, hf) = (f64::from(w), f64::from(h));
    let half_light = (hf / 10.0).round().max(1.0) / 2.0; // a diagonal's, from its middle
    match n {
        0x2504..=0x250B | 0x254C..=0x254F => {
            let k = if n >= 0x254C { n - 0x254C } else { n - 0x2504 };
            let parts = if n >= 0x254C { 2 } else { 3 + k / 4 };
            m.dashes(parts, 1 + k % 2, k % 4 >= 2, hf / 10.0);
        }
        0x256D..=0x2570 => {
            m.arc(hf / 10.0);
            m.flip(n == 0x256E || n == 0x256F, n == 0x256F || n == 0x2570);
        }
        0x2571..=0x2573 => {
            let len = wf.hypot(hf);
            let rising = |x: f64, y: f64| (hf * x + wf * y - wf * hf).abs() / len <= half_light;
            let falling = |x: f64, y: f64| (hf * x - wf * y).abs() / len <= half_light;
            m.shape(|x, y| (n != 0x2572 && rising(x, y)) || (n != 0x2571 && falling(x, y)));
        }
        0x2500..=0x257F => {
            let at = (n - 0x2500) as usize * 4;
            let arm = |i: usize| ARMS.as_bytes()[at + i] - b'0';
            m.arms([arm(0), arm(1), arm(2), arm(3)], hf / 10.0);
        }
        0x2580 => m.rect(0.0, 0.0, wf, hf / 2.0),
        0x2581..=0x2588 => m.rect(0.0, hf - hf * f64::from(n - 0x2580) / 8.0, wf, hf),
        0x2589..=0x258F => m.rect(0.0, 0.0, wf * f64::from(0x2590 - n) / 8.0, hf),
        0x2590 => m.rect(wf / 2.0, 0.0, wf, hf),
        0x2591..=0x2593 => m.px.fill(f64::from(n - 0x2590) / 4.0),
        0x2594 => m.rect(0.0, 0.0, wf, hf / 8.0),
        0x2595 => m.rect(wf - wf / 8.0, 0.0, wf, hf),
        0x2596..=0x259F => {
            let quads = [4, 8, 1, 13, 9, 7, 11, 2, 6, 14][(n - 0x2596) as usize];
            let (x, y) = (wf / 2.0, hf / 2.0);
            let corners = [(1, 0.0, 0.0), (2, x, 0.0), (4, 0.0, y), (8, x, y)];
            for (_, x0, y0) in corners.into_iter().filter(|it| quads & it.0 != 0) {
                m.rect(x0, y0, x0 + x, y0 + y);
            }
        }
        0xE0B0..=0xE0B3 => {
            let half = hf / 2.0;
            if n.is_multiple_of(2) {
                m.shape(|x, y| x <= wf * (1.0 - (y - half).abs() / half));
            } else {
                let len = wf.hypot(half);
                m.shape(|x, y| (half * x - wf * y.min(hf - y)).abs() / len <= half_light);
            }
            m.flip(n >= 0xE0B2, false);
        }
        _ => {}
    }
    m.px.iter().map(|&v| (v.clamp(0.0, 1.0) * 255.0).round() as u8).collect()
}

/// Coverage being drawn: a value a pixel, from 0 to 1.
struct Mask {
    w: usize,
    h: usize,
    px: Vec<f64>,
}

/// The nearest whole thickness to `ideal` with the parity of `side`, so a stroke centred
/// across `side` has both edges on pixel edges (the larger on a tie), at least 1.
fn thick(ideal: f64, side: usize) -> f64 {
    let near = ideal.round().max(1.0) as usize;
    if near % 2 == side % 2 {
        return near as f64;
    }
    let (lower, upper) = ((near - 1) as f64, (near + 1) as f64);
    let below = near > 1 && ideal - lower < upper - ideal;
    if below { (near - 1) as f64 } else { (near + 1) as f64 }
}

impl Mask {
    /// Adds the part of each pixel the rectangle covers.
    fn rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        let (cols, rows) = (
            x0.max(0.0) as usize..(x1.ceil() as usize).min(self.w),
            y0.max(0.0) as usize..(y1.ceil() as usize).min(self.h),
        );
        for y in rows {
            let dy = (y1.min(y as f64 + 1.0) - y0.max(y as f64)).max(0.0);
            for x in cols.clone() {
                let dx = (x1.min(x as f64 + 1.0) - x0.max(x as f64)).max(0.0);
                self.px[y * self.w + x] += dx * dy;
            }
        }
    }

    /// Sets each pixel to the part of its 8 x 8 samples, placed alike about its middle,
    /// `inside` holds for, where that is more.
    fn shape(&mut self, inside: impl Fn(f64, f64) -> bool) {
        for y in 0..self.h {
            for x in 0..self.w {
                let mut n = 0;
                for i in 0..8 {
                    for j in 0..8 {
                        let (sx, sy) = (x as f64 + (j as f64 + 0.5) / 8.0, y as f64 + (i as f64 + 0.5) / 8.0);
                        n += usize::from(inside(sx, sy));
                    }
                }
                let v = &mut self.px[y * self.w + x];
                *v = v.max(n as f64 / 64.0);
            }
        }
    }

    /// Mirrors the mask about its vertical middle (`x`) and its horizontal one (`y`).
    fn flip(&mut self, x: bool, y: bool) {
        let w = self.w;
        if x {
            self.px.chunks_exact_mut(w).for_each(<[f64]>::reverse);
        }
        if y {
            let rows: Vec<Vec<f64>> = self.px.chunks_exact(w).rev().map(<[f64]>::to_vec).collect();
            self.px = rows.concat();
        }
    }

    /// A transposed copy: rows become columns.
    fn transposed(&self) -> Mask {
        let mut px = vec![0.0; self.px.len()];
        for y in 0..self.h {
            for x in 0..self.w {
                px[x * self.h + y] = self.px[y * self.w + x];
            }
        }
        Mask { w: self.h, h: self.w, px }
    }

    /// Adds `other`, of the same size.
    fn add(&mut self, other: &Mask) {
        self.px.iter_mut().zip(&other.px).for_each(|(a, b)| *a += b);
    }

    /// The four arms, `[up, right, down, left]`: each horizontal arm drawn as the right one of
    /// the box mirrored, each vertical one as a horizontal one of the box transposed.
    fn arms(&mut self, [up, right, down, left]: [u8; 4], ideal: f64) {
        let (w, h) = (self.w, self.h);
        let mut one = |arms: [u8; 4], flip_x: bool, across: bool| {
            let mut m =
                Mask { w: if across { h } else { w }, h: if across { w } else { h }, px: vec![0.0; w * h] };
            m.right_arm(arms, ideal);
            m.flip(flip_x, false);
            self.add(&if across { m.transposed() } else { m });
        };
        // In a transposed box, right is down, left is up, up is left and down is right.
        one([up, right, down, left], false, false);
        one([up, left, down, right], true, false);
        one([left, down, right, up], false, true);
        one([left, up, right, down], true, true);
    }

    /// The right arm, if any, of `[up, right, down, left]`: from the box's right edge to
    /// where it meets the others (specs/18: a square, or a double line's corners).
    fn right_arm(&mut self, [up, right, down, left]: [u8; 4], ideal: f64) {
        let (w, h) = (self.w as f64, self.h as f64);
        let single = |weight: u8, side: usize| thick(ideal * f64::from(weight), side);
        // A double line across `side`: two light strokes and a gap as wide, so its whole width
        // has the side's parity too.
        let light = |side: usize| thick(ideal, side);
        let double = |side: usize| 3.0 * light(side);
        let width = |weight: u8, side: usize| match weight {
            0 => 0.0,
            3 => double(side),
            _ => single(weight, side),
        };
        match right {
            0 => {}
            3 => {
                let d = double(self.h);
                let stroke = light(self.h);
                for (top, y0) in [(true, (h - d) / 2.0), (false, (h + d) / 2.0 - stroke)] {
                    let (same, opposite) = if top { (up, down) } else { (down, up) };
                    let stop = match (same, opposite) {
                        (3, _) => (w + double(self.w)) / 2.0 - light(self.w),
                        (1 | 2, _) => (w + single(same, self.w)) / 2.0,
                        (_, 3) => (w - double(self.w)) / 2.0,
                        (_, 1 | 2) => (w - single(opposite, self.w)) / 2.0,
                        _ if left != 0 => 0.0,
                        _ => w / 2.0,
                    };
                    self.rect(stop, y0, w, y0 + stroke);
                }
            }
            weight => {
                let t = single(weight, self.h);
                let join = match up.max(down) {
                    0 => single(right.max(left).min(2), self.w),
                    _ => width(up, self.w).max(width(down, self.w)),
                };
                self.rect((w - join) / 2.0, (h - t) / 2.0, w, (h + t) / 2.0);
            }
        }
    }

    /// `parts` dashes of a light (1) or heavy (2) line, along the box or `down` it, each
    /// centred in an equal part with a third of it left empty. The parts are no whole pixels:
    /// the mask is made even with its mirror along the line, so rounding leaves it symmetric.
    fn dashes(&mut self, parts: u32, weight: u32, down: bool, ideal: f64) {
        let (w, h) = if down { (self.h, self.w) } else { (self.w, self.h) };
        let mut m = Mask { w, h, px: vec![0.0; self.px.len()] };
        let (w, h) = (m.w as f64, m.h as f64);
        let t = thick(ideal * f64::from(weight), m.h);
        let part = w / f64::from(parts);
        for k in 0..parts {
            let mid = (f64::from(k) + 0.5) * part;
            m.rect(mid - part / 3.0, (h - t) / 2.0, mid + part / 3.0, (h + t) / 2.0);
        }
        let mut mirrored = Mask { w: m.w, h: m.h, px: m.px.clone() };
        mirrored.flip(true, false);
        m.px.iter_mut().zip(&mirrored.px).for_each(|(a, b)| *a = (*a + b) / 2.0);
        self.add(&if down { m.transposed() } else { m });
    }

    /// `╭`: a quarter circle tangent to the lines from two edges' middles, the edge pixels theirs.
    fn arc(&mut self, ideal: f64) {
        let (w, h) = (self.w as f64, self.h as f64);
        let (tv, th) = (thick(ideal, self.w) / 2.0, thick(ideal, self.h) / 2.0);
        let r = (w.min(h) / 2.0 - 1.0).max(0.5);
        let (cx, cy) = (w / 2.0 + r, h / 2.0 + r);
        let e = |a: f64, b: f64, x: f64, y: f64| a.min(b) > 0.0 && (x / a).powi(2) + (y / b).powi(2) <= 1.0;
        self.shape(|x, y| {
            let (dx, dy) = (cx - x, cy - y);
            let ring = dx >= 0.0 && dy >= 0.0 && e(r + tv, r + th, dx, dy) && !e(r - tv, r - th, dx, dy);
            ring || ((x - w / 2.0).abs() <= tv && y >= cy) || ((y - h / 2.0).abs() <= th && x >= cx)
        });
    }
}
