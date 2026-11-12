//! Painting a drawing: boxes as quads, lines as strokes of the renderer (so a link of any
//! length is smooth and costs no atlas), filled shapes and the heads of arrows as masks in
//! the glyph atlas, text as glyphs.

use std::hash::{DefaultHasher, Hash, Hasher};

use quadra_kit::token;
use quadra_prim::{Color, Corners, Edges, Point, Rect};
use quadra_render::{Kind, Quad, Sprite, stroke};
use quadra_tree::{GlyphTile, Paint, PaintCx, Theme, paint_glyph};
use quadra_vector::{Coverage, Polyline};

use super::curve::dashes;
use super::draw::{Drawing, Head, Op};

/// The colour of each ink, in the order of `Ink`.
pub type Inks = [Color; 10];

pub fn inks(theme: &Theme) -> Inks {
    let of = |role, alpha| theme.resolve(Paint::Token(role, alpha));
    [
        Color::TRANSPARENT,
        of(token::PANEL, 1.0),
        of(token::CANVAS, 1.0),
        of(token::LINE_STRONG, 1.0),
        of(token::TEXT, 1.0),
        of(token::TEXT_2, 1.0),
        of(token::ACCENT, 1.0),
        of(token::ACCENT, 0.05),
        of(token::TEXT_ON_ACCENT, 1.0),
        of(token::ACCENT, 0.12),
    ]
}

/// The dashes one painting makes: past them a dashed line is drawn whole, so that a
/// document cannot ask for instances without end with lines of dots across a large chart.
const MOST_DASHES: usize = 20_000;

/// Paints the drawing with its corner at the corner of the node, on a whole pixel. The
/// tiles painted are added to `tiles`: holding them keeps them in the atlas.
pub fn paint(d: &Drawing, inks: &Inks, cx: &mut PaintCx, tiles: &mut Vec<GlyphTile>) {
    let (scale, clip) = (cx.scale, cx.clip);
    let (ox, oy) = ((cx.rect.x * scale).round(), (cx.rect.y * scale).round());
    let device = |p: &Point| Point::new(ox + p.x * scale, oy + p.y * scale);
    let mut dashed = 0;
    for op in &d.ops {
        match op {
            Op::Box { rect, radius, fill, border } => {
                let bounds = rect.translate(ox / scale, oy / scale);
                let border_widths =
                    if inks[*border as usize].a > 0.0 { Edges::all(1.0) } else { Edges::default() };
                let (background, border_color) = (inks[*fill as usize], inks[*border as usize]);
                cx.quad(Quad {
                    bounds,
                    background,
                    border_widths,
                    border_color,
                    radii: Corners::all(*radius),
                    ..Quad::default()
                });
            }
            Op::Line { points, width, dash, ink } => {
                let pieces = if *dash && dashed < MOST_DASHES {
                    dashes(points, 5.0, 4.0)
                } else {
                    vec![points.clone()]
                };
                dashed += pieces.len();
                for piece in pieces {
                    let piece: Vec<Point> = piece.iter().map(device).collect();
                    stroke(&piece, (width * scale).max(1.0), inks[*ink as usize], false, &clip, |i| {
                        cx.push(i)
                    });
                }
            }
            Op::Fill { points, fill, border } => {
                let points: Vec<Point> = points.iter().map(device).collect();
                tiles.extend(mask(cx, &points, inks[*fill as usize]));
                stroke(&points, scale.max(1.0), inks[*border as usize], true, &clip, |i| cx.push(i));
            }
            Op::Text { at, word, size, ink, .. } => {
                // The baseline is on a whole pixel.
                let (left, baseline) = (ox + at.x * scale, (oy + at.y * scale).round());
                for g in &word.glyphs {
                    let pen = Point::new(left + g.x * scale, baseline - (g.y * scale).round());
                    tiles.extend(paint_glyph(cx, (g.face, g.id), size * scale, pen, inks[*ink as usize]));
                }
            }
            Op::Head { tip, from, head, ink } => {
                tiles.extend(paint_head(cx, device(tip), device(from), *head, inks[*ink as usize]));
            }
        }
    }
}

/// A filled polygon, in device pixels, as a mask in the atlas: the points from the corner
/// of the mask, to a sixteenth of a pixel, are what the atlas knows it by.
// ponytail: a shape larger than a slice of the atlas has its outline and no fill; in bands
// when a node is ever that large.
fn mask(cx: &mut PaintCx, points: &[Point], color: Color) -> Option<GlyphTile> {
    let low = |of: fn(&Point) -> f32| points.iter().map(of).fold(f32::MAX, f32::min).floor() - 1.0;
    let high = |of: fn(&Point) -> f32| points.iter().map(of).fold(f32::MIN, f32::max).ceil() + 1.0;
    let (left, top) = (low(|p| p.x), low(|p| p.y));
    let (wide, tall) = (high(|p| p.x) - left, high(|p| p.y) - top);
    if points.len() < 3 || color.a <= 0.0 || !(wide * tall).is_finite() {
        return None;
    }
    let points = points.iter().map(|p| Point::new(p.x - left, p.y - top)).collect();
    let shape = Polyline { points, closed: true };
    let mut hasher = DefaultHasher::new();
    shape.points.iter().for_each(|p| ((p.x * 16.0) as i32, (p.y * 16.0) as i32).hash(&mut hasher));
    let (wide, tall) = (wide as u32, tall as u32);
    let tile = cx.fonts.mask(hasher.finish(), wide, tall, || {
        let mut coverage = Coverage::new(wide as usize, tall as usize);
        coverage.fill(std::slice::from_ref(&shape));
        coverage.to_bytes()
    })?;
    let bounds = Rect::new(left, top, tile.rect.w as f32, tile.rect.h as f32);
    let sprite = Sprite {
        bounds,
        uv: tile.uv,
        slice: tile.rect.slice,
        color,
        radii: Corners::default(),
        glyph: false,
    };
    cx.push(sprite.instance(Kind::Mono, &cx.clip));
    Some(tile)
}

/// A head with its tip at `tip`, on a line that comes from `from`: in device pixels.
fn paint_head(cx: &mut PaintCx, tip: Point, from: Point, head: Head, color: Color) -> Option<GlyphTile> {
    let (scale, clip) = (cx.scale, cx.clip);
    let length = (tip.x - from.x).hypot(tip.y - from.y);
    if length < 0.01 {
        return None;
    }
    // Along the line and across it, a logical pixel long.
    let (dx, dy) = ((tip.x - from.x) / length * scale, (tip.y - from.y) / length * scale);
    let at = |back: f32, side: f32| Point::new(tip.x - dx * back - dy * side, tip.y - dy * back + dx * side);
    let mut line = |a: Point, b: Point| stroke(&[a, b], 1.5 * scale, color, false, &clip, |i| cx.push(i));
    match head {
        Head::Arrow => {
            let half = length * 0.45;
            return mask(
                cx,
                &[tip, at(length / scale, half / scale), at(length / scale, -half / scale)],
                color,
            );
        }
        Head::Open => {
            line(at(9.0, 4.5), tip);
            line(at(9.0, -4.5), tip);
        }
        Head::Circle => {
            let (middle, r) = (at(4.0, 0.0), 4.0 * scale);
            let bounds = Rect::new(middle.x - r, middle.y - r, 2.0 * r, 2.0 * r);
            cx.push(
                Quad { bounds, background: color, radii: Corners::all(r), ..Quad::default() }.instance(&clip),
            );
        }
        Head::Cross => {
            line(at(9.0, 4.0), at(1.0, -4.0));
            line(at(9.0, -4.0), at(1.0, 4.0));
        }
        Head::None => {}
    }
    None
}
