//! Every drawn character checked against what its Unicode name says (`names.txt`, written
//! from the Unicode Character Database; the separators, private use, named by hand), on its
//! mask's pixels at each size of `SIZES`: the strokes at each edge (none, light, heavy,
//! double: how many, how thick, centred), nothing outside the arms, dashes, arcs, diagonals,
//! the eighths and quadrants of blocks, shades and the separators. An arm's edge pixels are
//! those of the straight line of its weight, so any two neighbours join.

use super::SIZES;
use crate::drawn::{drawn, mask};

mod says;
use says::{Says, says};

const NAMES: &str = include_str!("names.txt");

/// The nearest whole thickness to `ideal` with the parity of `side`, the larger on a tie, at
/// least 1 (specs/18, "Drawn characters").
fn stroke(ideal: f64, side: u32) -> u32 {
    let near = |t: &u32| ((f64::from(*t) - ideal).abs(), u32::MAX - t);
    (1..=side).filter(|t| t % 2 == side % 2).min_by(|a, b| near(a).partial_cmp(&near(b)).unwrap()).unwrap()
}

/// The strokes of a weight across a line of `len` pixels: for each pixel, whether it is in one.
fn profile(weight: u8, len: u32, h: u32) -> Vec<bool> {
    let ideal = f64::from(h) / 10.0;
    let light = stroke(ideal, len);
    let (total, parts): (u32, Vec<(u32, u32)>) = match weight {
        0 => (0, vec![]),
        3 => (3 * light, vec![(0, light), (2 * light, 3 * light)]),
        w => {
            let t = stroke(ideal * f64::from(w), len);
            (t, vec![(0, t)])
        }
    };
    let start = (len - total) / 2;
    (0..len).map(|i| parts.iter().any(|&(a, b)| (start + a..start + b).contains(&i))).collect()
}

/// The `i`th line of pixels from the edge of `side` (0 up, 1 right, 2 down, 3 left), across.
fn edge(m: &[u8], (w, h): (u32, u32), side: usize, i: u32) -> Vec<u8> {
    let at = |x: u32, y: u32| m[(y * w + x) as usize];
    match side {
        0 => (0..w).map(|x| at(x, i)).collect(),
        2 => (0..w).map(|x| at(x, h - 1 - i)).collect(),
        1 => (0..h).map(|y| at(w - 1 - i, y)).collect(),
        _ => (0..h).map(|y| at(i, y)).collect(),
    }
}

fn check_arms(c: char, m: &[u8], (w, h): (u32, u32), arms: [u8; 4], arc: bool) {
    let across = |side: usize| if side.is_multiple_of(2) { w } else { h };
    let want = |side: usize| profile(arms[side], across(side), h);
    for side in 0..4 {
        for i in 0..1 {
            let got: Vec<bool> = edge(m, (w, h), side, i).iter().map(|&v| v == 255).collect();
            let clean = edge(m, (w, h), side, i).iter().all(|&v| v == 0 || v == 255);
            assert!(
                clean && got == want(side),
                "{c} {w}x{h}: side {side}, line {i}: {:?}",
                edge(m, (w, h), side, i)
            );
        }
    }
    // Inside: the arms' strokes are whole up to where they meet; nothing lies outside them
    // and the box where they meet.
    let (cx, cy) = (f64::from(w) / 2.0, f64::from(h) / 2.0);
    // The width a pair of arms spans, from their first stroke to their last.
    let span = |s: usize| {
        let on = want(s);
        on.iter().rposition(|&b| b).map_or(0, |last| last + 1 - on.iter().position(|&b| b).unwrap())
    };
    let half = |sides: [usize; 2]| sides.map(span).into_iter().max().unwrap();
    if arc {
        return check_arc(c, m, (w, h), arms);
    }
    // Where arms meet is at least as wide as a line across of their weight: a lone arm ends
    // past the middle as that line would, so two of them make a corner.
    let across = |a: usize, b: usize, len: u32| {
        let on = profile(arms[a].max(arms[b]).max(1), len, h);
        on.iter().rposition(|&b| b).unwrap() + 1 - on.iter().position(|&b| b).unwrap()
    };
    let hv = f64::from(half([0, 2]).max(across(1, 3, w)) as u32) / 2.0;
    let hh = f64::from(half([1, 3]).max(across(0, 2, h)) as u32) / 2.0;
    for y in 0..h {
        for x in 0..w {
            let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let v = m[(y * w + x) as usize];
            let junction = (px - cx).abs() <= hv && (py - cy).abs() <= hh;
            let ins = [
                arms[0] > 0 && want(0)[x as usize] && py <= cy + hh,
                arms[1] > 0 && want(1)[y as usize] && px >= cx - hv,
                arms[2] > 0 && want(2)[x as usize] && py >= cy - hh,
                arms[3] > 0 && want(3)[y as usize] && px <= cx + hv,
            ];
            if !junction {
                let outside = ins.iter().any(|&it| it);
                assert_eq!(v, if outside { 255 } else { 0 }, "{c} {w}x{h} at ({x}, {y})");
            }
        }
    }
}

/// An arc: a quarter circle of half the shorter side less a pixel about a centre out towards
/// its two arms (specs/18), drawn through its middle, and nothing off it or its lines.
fn check_arc(c: char, m: &[u8], (w, h): (u32, u32), arms: [u8; 4]) {
    let (wf, hf) = (f64::from(w), f64::from(h));
    let r = (wf.min(hf) / 2.0 - 1.0).max(0.5);
    let (sx, sy) = (if arms[1] > 0 { 1.0 } else { -1.0 }, if arms[2] > 0 { 1.0 } else { -1.0 });
    let (ox, oy) = (wf / 2.0 + sx * r, hf / 2.0 + sy * r);
    let t = f64::from(stroke(hf / 10.0, w).max(stroke(hf / 10.0, h)));
    let at = |x: f64, y: f64| m[(y as u32 * w + x as u32) as usize];
    let mid = std::f64::consts::FRAC_1_SQRT_2 * r;
    assert!(at(ox - sx * mid, oy - sy * mid) >= 100, "{c} {w}x{h}: no stroke at the arc's middle");
    for y in 0..h {
        for x in 0..w {
            let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let ring = ((px - ox).hypot(py - oy) - r).abs() <= t / 2.0 + 1.0
                && (ox - px) * sx >= -1.0
                && (oy - py) * sy >= -1.0;
            let down = (px - wf / 2.0).abs() <= t / 2.0 + 0.5 && (py - oy) * sy >= -1.0;
            let across = (py - hf / 2.0).abs() <= t / 2.0 + 0.5 && (px - ox) * sx >= -1.0;
            let v = m[(y * w + x) as usize];
            assert!(v == 0 || ring || down || across, "{c} {w}x{h}: a stray pixel at ({x}, {y}): {v}");
        }
    }
}

/// Runs of pixels at least half covered along a line.
fn runs(line: &[u8]) -> usize {
    line.iter()
        .zip(std::iter::once(&0).chain(line))
        .filter(|(v, before)| **v > 128 && **before <= 128)
        .count()
}

fn check(c: char, says: &Says, (w, h): (u32, u32)) {
    let m = mask(c, w, h);
    assert!(m.iter().any(|&v| v > 0), "{c} {w}x{h} draws nothing");
    let at = |x: u32, y: u32| m[(y * w + x) as usize];
    match *says {
        Says::Arms(arms, arc) => check_arms(c, &m, (w, h), arms, arc),
        Says::Dashes(n, weight, across) => {
            let (len, other) = if across { (w, h) } else { (h, w) };
            let line: Vec<u8> = (0..len).map(|i| if across { at(i, h / 2) } else { at(w / 2, i) }).collect();
            if len / n as u32 >= 3 {
                assert_eq!(runs(&line), n, "{c} {w}x{h}: {line:?}");
            }
            let band = profile(weight, other, h);
            // Across the line where a dash covers most: its strokes, whole pixels.
            let peak = *line.iter().max().unwrap();
            let first = line.iter().position(|&v| v == peak).unwrap() as u32;
            let through: Vec<bool> =
                (0..other).map(|j| if across { at(first, j) } else { at(j, first) } == peak).collect();
            assert_eq!(through, band, "{c} {w}x{h}: thickness");
        }
        Says::Diagonal(rising, falling) => {
            let corners = [
                (at(0, h - 1), rising),
                (at(w - 1, 0), rising),
                (at(0, 0), falling),
                (at(w - 1, h - 1), falling),
            ];
            assert!(corners.iter().all(|&(v, want)| (v > 0) == want), "{c} {w}x{h}: corners {corners:?}");
            assert!((0..h).all(|y| (0..w).any(|x| at(x, y) > 0)), "{c} {w}x{h}: a row without the line");
        }
        Says::Blocks(ref parts) => {
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
                    assert!(
                        (f64::from(at(x, y)) - want).abs() <= 1.0,
                        "{c} {w}x{h} at ({x}, {y}): {} for {want}",
                        at(x, y)
                    );
                }
            }
        }
        Says::Shade(part) => {
            assert!(m.iter().all(|&v| (f64::from(v) - part * 255.0).abs() <= 1.0), "{c} {w}x{h}")
        }
        Says::Separator(right, solid) => {
            let (near, far) = if right { (0, w - 1) } else { (w - 1, 0) };
            let area = m.iter().map(|&v| f64::from(v)).sum::<f64>() / (255.0 * f64::from(w * h));
            assert!(at(near, 0) > 0 && at(near, h - 1) > 0 && at(far, h / 2) > 0, "{c} {w}x{h}: its points");
            assert_eq!((at(far, 0), at(far, h - 1)), (0, 0), "{c} {w}x{h}: the far corners");
            if solid {
                assert!(
                    (area - 0.5).abs() < 0.03 && (1..h - 1).all(|y| at(near, y) >= 250),
                    "U+{:04X} {w}x{h}: {area}",
                    c as u32
                );
            } else {
                assert!(area < 0.4 && at(near, h / 2) == 0, "U+{:04X} {w}x{h}: a chevron, {area}", c as u32);
            }
        }
    }
}

#[test]
fn every_drawn_character_is_what_its_unicode_name_says_at_every_size() {
    let named: Vec<(char, Says)> = NAMES
        .lines()
        .map(|line| {
            let (code, name) = line.split_once(' ').expect("a code and a name");
            (char::from_u32(u32::from_str_radix(code, 16).unwrap()).unwrap(), says(name))
        })
        .collect();
    // Enough: every character drawn here has a name, and every named one is drawn.
    let all: Vec<char> = (0x2500..0x25A0).chain(0xE0B0..0xE0B4).filter_map(char::from_u32).collect();
    assert_eq!(named.iter().map(|it| it.0).collect::<Vec<_>>(), all);
    assert!(all.iter().all(|&c| drawn(c)) && !drawn('\u{24FF}') && !drawn('\u{25A0}') && !drawn('\u{E0B4}'));
    for (c, says) in &named {
        for size in SIZES {
            check(*c, says, size);
        }
    }
}
