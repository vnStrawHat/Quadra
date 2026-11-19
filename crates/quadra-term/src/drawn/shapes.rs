//! Sextants and the separators of patched fonts: each a shape drawn once and mirrored, its
//! straight sides on the box's edges, so it meets its neighbours along them.

use super::Mask;

impl Mask {
    /// Sextant `k` (U+1FB00 + `k`): parts of two columns, the half blocks', by three rows split
    /// at whole pixels, `1 2 / 3 4 / 5 6`; filled as the bits of a number from 1 to 62 that
    /// skips the two half blocks (`1 3 5` and `2 4 6`), the order of their code points.
    pub(super) fn sextant(&mut self, k: u32) {
        let (w, h) = (self.w as f64, self.h as f64);
        let parts = k + 1 + u32::from(k >= 20) + u32::from(k >= 40);
        let rows = [0.0, (h / 3.0).round(), (h * 2.0 / 3.0).round(), h];
        for part in (0..6).filter(|p| parts >> p & 1 == 1) {
            let (x, row) = (f64::from(part % 2) * w / 2.0, part as usize / 2);
            self.rect(x, rows[row], x + w / 2.0, rows[row + 1]);
        }
    }

    /// Separator `k` (U+E0B0 + `k`), an outline a stroke of a diagonal's weight (`half` is
    /// half of it): a triangle and a chevron pointing right, a half ellipse on the left edge
    /// as wide as the box and its arc, a triangle below the falling diagonal and that
    /// diagonal; the rest their mirrors.
    pub(super) fn powerline(&mut self, k: u32, half: f64) {
        let (w, h) = (self.w as f64, self.h as f64);
        let (mid, solid) = (h / 2.0, k.is_multiple_of(2));
        match k {
            0..=3 if solid => self.shape(|x, y| x <= w * (1.0 - (y - mid).abs() / mid)),
            0..=3 => {
                let len = w.hypot(mid);
                self.shape(|x, y| (mid * x - w * y.min(h - y)).abs() / len <= half);
            }
            4..=7 => {
                let within = |a: f64, b: f64, x: f64, y: f64| {
                    a.min(b) > 0.0 && (x / a).powi(2) + ((y - mid) / b).powi(2) <= 1.0
                };
                let t = 2.0 * half;
                self.shape(|x, y| within(w, mid, x, y) && (solid || !within(w - t, mid - t, x, y)));
            }
            _ if solid => self.shape(|x, y| y * w >= x * h),
            _ => {
                let len = w.hypot(h);
                self.shape(|x, y| (h * x - w * y).abs() / len <= half);
            }
        }
        self.flip(k & 2 != 0, k >= 8 && k & 4 != 0);
    }
}
