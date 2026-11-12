//! Strokes: a line through points, an instance a segment (specs/07, "The instance"). The
//! shader shades each pixel by how far it is from its segment, so a stroke of any slope and
//! any length is smooth, has round ends and joints, and nothing is rasterized on the CPU.

use quadra_prim::{Color, Point, Rect};

use crate::instance::{Clip, Instance, Kind};

/// What `params[1]` of a line says for a segment of a stroke.
const SEGMENT: f32 = 3.0;
/// The segment has one before it, and one after it: a pixel at a joint is drawn by the
/// nearer of the two, so a joint is covered once and a stroke that is not opaque is even.
const HAS_BEFORE: u32 = 1;
const HAS_AFTER: u32 = 2;

/// Gives `push` the instances of a line through `points`, `width` wide, in device pixels.
/// A closed one goes back from its last point to its first.
pub fn stroke(
    points: &[Point],
    width: f32,
    color: Color,
    closed: bool,
    clip: &Clip,
    mut push: impl FnMut(Instance),
) {
    let count = points.len();
    let segments = if closed && count > 2 { count } else { count.saturating_sub(1) };
    // The point `k` places on from the start of segment `i`: none past an open end.
    let at = |i: usize, k: isize| -> Option<Point> {
        let j = i as isize + k;
        let inside = (0..count as isize).contains(&j);
        (inside || segments == count).then(|| points[j.rem_euclid(count as isize) as usize])
    };
    let reach = width / 2.0 + 1.0;
    for i in 0..segments {
        let (Some(a), Some(b)) = (at(i, 0), at(i, 1)) else { continue };
        let (left, top) = ((a.x.min(b.x) - reach).floor(), (a.y.min(b.y) - reach).floor());
        let (right, bottom) = ((a.x.max(b.x) + reach).ceil(), (a.y.max(b.y) + reach).ceil());
        let bounds = Rect::new(left, top, right - left, bottom - top);
        let mut instance = Instance::base(Kind::Line, bounds, clip);
        let (before, after) = (at(i, -1), at(i, 2));
        instance.color = color.to_array();
        instance.radii = [a.x, a.y, b.x, b.y];
        instance.uv =
            [before.unwrap_or(a).x, before.unwrap_or(a).y, after.unwrap_or(b).x, after.unwrap_or(b).y];
        let (has_before, has_after) = (u32::from(before.is_some()), u32::from(after.is_some()));
        instance.flags = (has_before * HAS_BEFORE) | (has_after * HAS_AFTER);
        (instance.params[0], instance.params[1]) = (width, SEGMENT);
        push(instance);
    }
}
