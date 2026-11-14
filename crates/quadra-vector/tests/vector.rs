//! Paths, coverage and the SVG reader: areas come out exact, edges at half pixels are half
//! covered, an icon reads and renders, and nothing a file can hold makes any of it fail.

use quadra_prim::{Color, Point};
use quadra_vector::{Coverage, Path, Svg, numbers};

fn sum(pixels: &[u8]) -> u32 {
    pixels.iter().map(|&p| p as u32).sum()
}

fn filled(path: &Path, width: usize, height: usize, tolerance: f32) -> Vec<u8> {
    let mut coverage = Coverage::new(width, height);
    coverage.fill(&path.flatten(1.0, Point::ZERO, tolerance));
    coverage.to_bytes()
}

#[test]
fn a_filled_square_has_its_exact_area_and_half_pixels_are_half_covered() {
    let mut square = Path::new();
    square.rect(2.0, 2.0, 4.0, 4.0);
    let pixels = filled(&square, 10, 10, 0.25);
    assert_eq!((pixels[3 * 10 + 3], pixels[0], sum(&pixels)), (255, 0, 16 * 255));

    let mut bar = Path::new();
    bar.rect(0.5, 0.0, 2.0, 4.0); // half of column 0 and half of column 2
    assert_eq!(filled(&bar, 4, 4, 0.25)[..4], [128, 255, 128, 0]);

    let mut circle = Path::new();
    circle.ellipse(20.0, 20.0, 15.0, 15.0);
    let area = sum(&filled(&circle, 40, 40, 0.1)) as f32 / 255.0;
    assert!((702.0..712.0).contains(&area), "{area}"); // pi * 15 * 15 is 706.9
}

#[test]
fn a_shape_cut_by_the_sides_of_a_bitmap_is_covered_as_in_a_larger_one() {
    let triangle = |dx: f32| {
        let mut path = Path::new();
        path.move_to(1.0 + dx, 1.0);
        path.line_to(19.0 + dx, 4.0);
        path.line_to(7.0 + dx, 19.0);
        path.close();
        path
    };
    let whole = filled(&triangle(0.0), 20, 20, 0.25);
    // The middle columns of it, from a bitmap that starts at column 6 and ends at 14.
    let cut = filled(&triangle(-6.0), 8, 20, 0.25);
    for y in 0..20 {
        let (a, b) = (&cut[y * 8..y * 8 + 8], &whole[y * 20 + 6..y * 20 + 14]);
        assert!(a.iter().zip(b).all(|(a, b)| a.abs_diff(*b) <= 1), "row {y}: {a:?} {b:?}");
    }
}

#[test]
fn a_stroke_is_as_wide_as_it_is_told() {
    let mut line = Path::new();
    line.move_to(2.0, 5.0);
    line.line_to(18.0, 5.0);
    let mut coverage = Coverage::new(20, 10);
    coverage.stroke(&line.flatten(1.0, Point::ZERO, 0.25), 2.0);
    let pixels = coverage.to_bytes();
    assert_eq!((pixels[5 * 20 + 10], pixels[4 * 20 + 10], pixels[2 * 20 + 10]), (255, 255, 0));
}

#[test]
fn numbers_and_path_data_are_read() {
    assert_eq!(numbers("1,2 -3.5e1 .5-1"), [1.0, 2.0, -35.0, 0.5, -1.0]);
    let mut path = Path::new();
    path.push_data("M0 0h10v10H0z");
    let lines = path.flatten(1.0, Point::ZERO, 0.25);
    assert_eq!((lines.len(), lines[0].closed, lines[0].points.len()), (1, true, 4));
    // The same shape has the same number, and another shape another.
    let mut again = Path::new();
    again.push_data("M0 0 h10 v10 H0 Z");
    let mut other = Path::new();
    other.push_data("M0 0h10v11H0z");
    assert!(path.hash() == again.hash() && path.hash() != other.hash());
}

#[test]
fn an_icon_is_read_and_rendered() {
    // Stroked paths, a circle, and an arc whose flags are written without a space.
    let icon = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"
        fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/><path d="M3 12a9 9 0 0118 0"/></svg>"#;
    let svg = Svg::parse(icon);
    assert_eq!((svg.shapes.len(), svg.view), (3, (0.0, 0.0, 24.0, 24.0)));
    assert!(svg.shapes[0].current && svg.shapes[0].fill.is_none()); // fill="none" came from the svg
    assert!(svg.is_single_color());
    let mask = svg.render_mask(48, 48);
    assert_eq!(mask.len(), 48 * 48);
    assert!(sum(&mask) > 255 * 200); // visibly drawn
    assert_eq!(mask[22 * 48 + 22], 0); // the lens is hollow

    let art = Svg::parse(r##"<svg viewBox="0 0 4 4"><rect width="4" height="4" fill="#f00"/></svg>"##);
    assert!(!art.is_single_color());
    assert_eq!(art.render_rgba(2, 2, Color::WHITE)[..4], [255, 0, 0, 255]);
}

/// A small deterministic generator: the same inputs on every run.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n.max(1)
    }
}

const CORPUS: [&str; 6] = [
    r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 12h14M12 5l7 7-7 7"/></svg>"#,
    r##"<svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="6" fill="#f00"/><rect x="1" y="1" width="4" height="4" rx="1"/></svg>"##,
    r##"<?xml version="1.0"?><!-- c --><svg viewBox="-2 -2 20 20"><g fill="#0af" stroke="white"><ellipse cx="5" cy="5" rx="4" ry="2"/><line x1="0" y1="0" x2="9" y2="9"/></g></svg>"##,
    r##"<svg><polygon points="1,1 9,1 5,8"/><polyline points="0 0 4 4 8 0" stroke="#abc" fill="none"/></svg>"##,
    r#"<svg viewBox="0 0 24 24"><path d="M2 2C5 0 9 0 12 2S19 4 22 2Q20 8 22 12T22 20a4 4 0 1 1-8 0A6 3 30 011 4 4zH2V10z" fill="currentColor"/></svg>"#,
    r#"<svg viewBox="0 0 10 10"><path d="m1 1 2 2 3-3l.5.5.5-.5e1 1e-1 1E+1h-1v-1c1 1 1 1 1 1s1 1 1 1q1 1 1 1t1 1z"/></svg>"#,
];

const EDGES: [&str; 8] = ["", "0", "-0", "1e38", "-1e38", "1e-45", "NaN", "inf"];

fn check(source: &str) {
    let svg = Svg::parse(source);
    let _ = (svg.render_mask(16, 16), svg.render_rgba(12, 9, Color::WHITE), svg.is_single_color());
    for shape in &svg.shapes {
        let _ = (shape.path.hash(), shape.path.flatten(3.5, Point::new(-1.0, 2.0), 0.1));
    }
}

#[test]
fn nothing_a_file_can_hold_makes_reading_or_rendering_fail() {
    CORPUS.iter().for_each(|source| check(source));
    let mut rng = Rng(11);
    for round in 0..400 {
        // A drawing of the corpus with some of its bytes changed, cut, or doubled.
        let mut bytes = CORPUS[rng.below(CORPUS.len())].as_bytes().to_vec();
        for _ in 0..1 + rng.below(8) {
            let at = rng.below(bytes.len());
            match rng.below(4) {
                0 => bytes[at] = b" <>/=\"'-.eEaAzZmM0123456789#"[rng.below(28)],
                1 => bytes.truncate(at.max(1)),
                2 => bytes.insert(at, bytes[at]),
                _ => bytes[at] = rng.below(256) as u8,
            }
        }
        check(&String::from_utf8_lossy(&bytes));
        // And shapes of their own kind: deep, far, long.
        let n = 50 + rng.below(1500);
        let made = match round % 5 {
            0 => format!(
                "<svg>{}<path d=\"M0 0L5 5Z\"/>{}",
                "<g fill=\"#123\">".repeat(n),
                "</g>".repeat(rng.below(n))
            ),
            1 => format!(
                "<svg><path d=\"M0 0{}Z\"/></svg>",
                " L1e30 -1e30 a1e30 1 0 1 1 1e-30 1e38".repeat(n / 10)
            ),
            2 => format!("<svg><polygon points=\"{} 0 0 {}\"/></svg>", "9".repeat(n), "1 ".repeat(n)),
            3 => format!(
                "<svg viewBox=\"{} 0 {} {}\"><path d=\"M0 0h9v9z\" stroke-width=\"{}\" stroke=\"#fff\"/></svg>",
                EDGES[rng.below(8)],
                EDGES[rng.below(8)],
                EDGES[rng.below(8)],
                EDGES[rng.below(8)]
            ),
            _ => format!("{}={}>", "<".repeat(n), "\"".repeat(n)),
        };
        check(&made);
    }
}
