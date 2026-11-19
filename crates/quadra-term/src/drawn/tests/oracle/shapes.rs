//! The checks of blocks, sextants, the separators' half circles and corner triangles, and of
//! diagonals keeping to their lines.

/// The light weight of a stroke not centred on the box's middle (a diagonal's, an outline's).
fn light(h: u32) -> f64 {
    (f64::from(h) / 10.0).round().max(1.0)
}

/// Each pixel covered by its part of the rectangles `parts` (fractions of the box), to 1 in 255.
pub(super) fn blocks(c: char, m: &[u8], (w, h): (u32, u32), parts: &[[f64; 4]]) {
    let (wf, hf) = (f64::from(w), f64::from(h));
    for y in 0..h {
        for x in 0..w {
            let covered: f64 = parts
                .iter()
                .map(|&[x0, y0, x1, y1]| {
                    let dx = (x1 * wf).min(f64::from(x + 1)) - (x0 * wf).max(f64::from(x));
                    let dy = (y1 * hf).min(f64::from(y + 1)) - (y0 * hf).max(f64::from(y));
                    dx.max(0.0) * dy.max(0.0)
                })
                .sum();
            let want = (covered.min(1.0) * 255.0).round();
            let v = m[(y * w + x) as usize];
            assert!((f64::from(v) - want).abs() <= 1.0, "{c} {w}x{h} at ({x}, {y}): {v} for {want}");
        }
    }
}

/// A sextant: the columns of the half blocks, the rows split at the whole pixels nearest a
/// third and two thirds of the height, so the three rows tile it and differ by a pixel at most.
pub(super) fn sextant(c: char, m: &[u8], (w, h): (u32, u32), bits: u8) {
    let hf = f64::from(h);
    let rows = [0.0, (hf / 3.0).round(), (hf * 2.0 / 3.0).round(), hf];
    let tall: Vec<f64> = rows.windows(2).map(|it| it[1] - it[0]).collect();
    let (low, high) = tall.iter().fold((hf, 0.0), |(a, b), &t| (t.min(a), t.max(b)));
    assert!(low >= 1.0 && high - low <= 1.0, "{c} {w}x{h}: rows {tall:?}");
    let parts: Vec<[f64; 4]> = (0..6)
        .filter(|p| bits >> p & 1 == 1)
        .map(|p| {
            let (x, row) = (f64::from(p % 2) / 2.0, p as usize / 2);
            [x, rows[row] / hf, x + 0.5, rows[row + 1] / hf]
        })
        .collect();
    blocks(c, m, (w, h), &parts);
}

/// A half circle: half an ellipse on the flat edge as wide as the box and as tall, the same
/// mirrored top to bottom; solid, the flat edge covered its whole height; its arc, a stroke of
/// the light weight across the middle and at the flat edge, hollow inside.
pub(super) fn half_circle(c: char, m: &[u8], (w, h): (u32, u32), right: bool, solid: bool) {
    let at = |x: u32, y: u32| m[(y * w + if right { x } else { w - 1 - x }) as usize];
    let name = format!("U+{:04X} {w}x{h}", c as u32);
    assert!((0..h).all(|y| (0..w).all(|x| at(x, y) == at(x, h - 1 - y))), "{name}: not symmetric");
    assert!(at(w - 1, h / 2) > 0 && at(w - 1, 0) == 0, "{name}: its far point and corner");
    let (wf, mid) = (f64::from(w), f64::from(h) / 2.0);
    let inside = |a: f64, b: f64, x: u32, y: u32| {
        ((f64::from(x) + 0.5) / a).powi(2) + ((f64::from(y) + 0.5 - mid) / b).powi(2) <= 1.0
    };
    let t = light(h);
    for y in 0..h {
        for x in 0..w {
            let v = at(x, y);
            assert!(v == 0 || inside(wf + 1.0, mid + 1.0, x, y), "{name}: ({x}, {y}) outside");
            if solid {
                assert!(v == 255 || !inside(wf - 1.0, mid - 1.0, x, y), "{name}: ({x}, {y}) not filled");
            } else {
                assert!(v == 0 || !inside(wf - t - 1.0, mid - t - 1.0, x, y), "{name}: ({x}, {y}) inside");
            }
        }
    }
    if solid {
        let area = m.iter().map(|&v| f64::from(v)).sum::<f64>() / (255.0 * f64::from(w * h));
        assert!((area - std::f64::consts::FRAC_PI_4).abs() < 0.03, "{name}: {area}");
        assert!(at(0, 0) >= 128 && (1..h - 1).all(|y| at(0, y) == 255), "{name}: the flat edge");
    } else {
        let across = (0..w).filter(|&x| at(x, h / 2) >= 128).count() as f64;
        let down = (0..h / 2).filter(|&y| at(0, y) >= 128).count() as f64;
        assert!((across - t).abs() <= 1.0 && (down - t).abs() <= 1.0, "{name}: stroke {across}, {down}");
        assert_eq!(at(0, h / 2), 0, "{name}: hollow");
    }
}

/// A corner triangle: each pixel covered by its part of the half of the box on its side of
/// the diagonal between the two other corners, to a sixteenth (what 8 x 8 samples allow).
pub(super) fn corner(c: char, m: &[u8], (w, h): (u32, u32), right: bool, lower: bool) {
    let (wf, hf) = (f64::from(w), f64::from(h));
    for y in 0..h {
        for x in 0..w {
            // Down 64 lines of the pixel, the part each covers exactly.
            let covered: f64 = (0..64)
                .map(|j| {
                    let sx = f64::from(x) + (f64::from(j) + 0.5) / 64.0;
                    let edge = if right { wf - sx } else { sx } * hf / wf;
                    let (top, bottom) = if lower { (edge, hf) } else { (0.0, hf - edge) };
                    (bottom.min(f64::from(y + 1)) - top.max(f64::from(y))).max(0.0)
                })
                .sum::<f64>()
                / 64.0;
            let v = f64::from(m[(y * w + x) as usize]);
            assert!(
                (v - covered * 255.0).abs() <= 255.0 / 16.0 + 2.0,
                "U+{:04X} {w}x{h} at ({x}, {y}): {v} for {}",
                c as u32,
                covered * 255.0
            );
        }
    }
}

/// Nothing covered farther than half the light weight and a pixel from the diagonals drawn.
pub(super) fn on_diagonals(c: char, m: &[u8], (w, h): (u32, u32), rising: bool, falling: bool) {
    let (wf, hf) = (f64::from(w), f64::from(h));
    let (len, reach) = (wf.hypot(hf), light(h) / 2.0 + 1.0);
    for y in 0..h {
        for x in 0..w {
            let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let near_rising = rising && (hf * px + wf * py - wf * hf).abs() / len <= reach;
            let near_falling = falling && (hf * px - wf * py).abs() / len <= reach;
            let v = m[(y * w + x) as usize];
            assert!(
                v == 0 || near_rising || near_falling,
                "U+{:04X} {w}x{h}: ({x}, {y}) off its line",
                c as u32
            );
        }
    }
}
