//! Minimum contrast (specs/18, "The view"): a glyph's colour moved toward white or black until
//! it stands out from its background by a ratio; worked out once a pair of colours.

use std::collections::HashMap;

use quadra_prim::Color;

/// The WCAG 2 contrast of two opaque colours, from 1 to 21.
pub fn contrast_ratio(a: Color, b: Color) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

fn luminance(c: Color) -> f32 {
    let lin = |v: f32| if v <= 0.040_45 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
}

/// `fg` moved toward white or black, the nearer of the two that reaches `min` against `bg`
/// (the one that gets closest when neither does); `fg` itself when it already does.
pub fn with_contrast(fg: Color, bg: Color, min: f32) -> Color {
    if min <= 1.0 || contrast_ratio(fg, bg) >= min {
        return fg;
    }
    // The least of the way to `end` that reaches `min`, or the whole way.
    let toward = |end: Color| -> (Option<f32>, Color) {
        let reach = |t: f32| contrast_ratio(fg.lerp(end, t), bg) >= min;
        if !reach(1.0) {
            return (None, end);
        }
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..12 {
            let mid = (lo + hi) / 2.0;
            if reach(mid) { hi = mid } else { lo = mid }
        }
        (Some(hi), fg.lerp(end, hi))
    };
    let white = Color { a: fg.a, ..Color::WHITE };
    let black = Color { a: fg.a, ..Color::BLACK };
    match (toward(white), toward(black)) {
        ((Some(up), lighter), (Some(down), darker)) => {
            if up <= down {
                lighter
            } else {
                darker
            }
        }
        ((Some(_), lighter), _) => lighter,
        (_, (Some(_), darker)) => darker,
        _ => {
            if contrast_ratio(white, bg) >= contrast_ratio(black, bg) {
                white
            } else {
                black
            }
        }
    }
}

/// The colours worked out for the pairs the paint met, at one ratio.
#[derive(Default)]
pub(crate) struct Contrast {
    min: f32,
    memo: HashMap<(u32, u32), Color>,
    /// The pair asked last: cells in a row mostly share their colours.
    last: Option<((u32, u32), Color)>,
}

/// A pair's key: the bytes of its colours.
fn key(c: Color) -> u32 {
    let rgb = crate::widget::rgb(c);
    u32::from(rgb.r) << 16 | u32::from(rgb.g) << 8 | u32::from(rgb.b)
}

impl Contrast {
    pub(crate) fn set(&mut self, min: f32) {
        if min != self.min {
            (self.min, self.memo, self.last) = (min, HashMap::new(), None);
        }
    }

    /// `fg` over the opaque `bg`, moved until the ratio holds.
    pub(crate) fn fg(&mut self, fg: Color, bg: Color) -> Color {
        if self.min <= 1.0 {
            return fg;
        }
        let pair = (key(fg), key(bg));
        if let Some((last, color)) = self.last
            && last == pair
        {
            return color;
        }
        if self.memo.len() >= 1024 {
            self.memo.clear(); // a screen of 24-bit backgrounds; the usual pairs are a few
        }
        let min = self.min;
        let color = *self.memo.entry(pair).or_insert_with(|| with_contrast(fg, bg, min));
        self.last = Some((pair, color));
        color
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dim_colour_is_moved_until_the_ratio_holds_and_a_bright_one_is_left() {
        let (bg, dim) = (Color::rgb(0x1E1E1E), Color::rgb(0x3A3A6A));
        let moved = with_contrast(dim, bg, 4.5);
        assert!(contrast_ratio(moved, bg) >= 4.5, "{}", contrast_ratio(moved, bg));
        assert!(moved.b > moved.r, "the hue is kept: {moved:?}");
        assert_eq!(with_contrast(Color::WHITE, bg, 4.5), Color::WHITE);
        assert_eq!(with_contrast(dim, bg, 1.0), dim, "1 is off");
        let grey = Color::rgb(0x777777);
        assert!(contrast_ratio(with_contrast(grey, grey, 21.0), grey) > 4.0, "the most it can");
    }
}
