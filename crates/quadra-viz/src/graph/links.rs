//! The links of a graph: where a curve from one node to another runs, its dashes and its
//! head, and the widget that paints them all as strokes.

use std::rc::Rc;

use quadra_layout::{Known, Length, Position};
use quadra_prim::{Point, Rect, Size};
use quadra_reactive::Signal;
use quadra_render::stroke;
use quadra_tree::{MeasureCx, NodeId, PaintCx, Style, Theme, Widget};
use quadra_view::{MountCx, View};

use super::{GraphView, LinkStyle, Styles};

/// How many straight pieces a curve is drawn with.
const PIECES: usize = 24;
/// The head: how long its two strokes are, and how far they open.
const HEAD: f32 = 7.0;
const OPEN: f32 = 0.5;
/// The dashes of one link, past which it is drawn whole.
const MOST_DASHES: usize = 2_000;

/// The curve from the edge of `from` through the points `through` to the edge of `to`, along
/// the axis the nodes' middles are further apart on, in the plane the rects are in: a cubic
/// from one stop to the next, square to the axis at each.
pub fn route(from: Rect, to: Rect, through: &[Point]) -> Vec<Point> {
    let (a, b) = (from.center(), to.center());
    let across = (b.x - a.x).abs() >= (b.y - a.y).abs();
    // It leaves toward its first stop and arrives from its last.
    let (first, last) = (through.first().copied().unwrap_or(b), through.last().copied().unwrap_or(a));
    let start = match across {
        true if first.x >= a.x => Point::new(from.right(), a.y),
        true => Point::new(from.x, a.y),
        false if first.y >= a.y => Point::new(a.x, from.bottom()),
        false => Point::new(a.x, from.y),
    };
    let end = match across {
        true if b.x >= last.x => Point::new(to.x, b.y),
        true => Point::new(to.right(), b.y),
        false if b.y >= last.y => Point::new(b.x, to.y),
        false => Point::new(b.x, to.bottom()),
    };
    let stops: Vec<Point> = std::iter::once(start).chain(through.iter().copied()).chain([end]).collect();
    let mut line = vec![start];
    for pair in stops.windows(2) {
        let (s, e) = (pair[0], pair[1]);
        // The handles pull half the way along the axis.
        let (c1, c2) = match across {
            true => {
                let pull = (e.x - s.x) / 2.0;
                (Point::new(s.x + pull, s.y), Point::new(e.x - pull, e.y))
            }
            false => {
                let pull = (e.y - s.y) / 2.0;
                (Point::new(s.x, s.y + pull), Point::new(e.x, e.y - pull))
            }
        };
        line.extend((1..=PIECES).map(|k| cubic([s, c1, c2, e], k as f32 / PIECES as f32)));
    }
    line
}

fn cubic([a, b, c, d]: [Point; 4], t: f32) -> Point {
    let u = 1.0 - t;
    let (wa, wb, wc, wd) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    Point::new(wa * a.x + wb * b.x + wc * c.x + wd * d.x, wa * a.y + wb * b.y + wc * c.y + wd * d.y)
}

/// The two strokes of a head at the end of a line: a wing, the tip, the other wing.
pub fn head(points: &[Point], width: f32) -> Option<[Point; 3]> {
    let [.., from, tip] = points else { return None };
    let (dx, dy) = (tip.x - from.x, tip.y - from.y);
    let length = dx.hypot(dy);
    if length == 0.0 {
        return None;
    }
    let (ux, uy, long) = (dx / length, dy / length, HEAD + width);
    let wing = |side: f32| {
        let (sx, sy) = (-uy * side * OPEN, ux * side * OPEN);
        Point::new(tip.x - long * (ux - sx), tip.y - long * (uy - sy))
    };
    Some([wing(1.0), *tip, wing(-1.0)])
}

/// The pieces of a line that are drawn, `dash` long with as much between them; the whole
/// line when it is not dashed.
pub fn dashes(points: &[Point], dash: f32) -> Vec<Vec<Point>> {
    if dash <= 0.0 {
        return vec![points.to_vec()];
    }
    let (mut all, mut piece, mut left, mut on) = (Vec::new(), vec![points[0]], dash, true);
    for pair in points.windows(2) {
        let (mut a, b) = (pair[0], pair[1]);
        let mut length = (b.x - a.x).hypot(b.y - a.y);
        while length > left && all.len() < MOST_DASHES {
            let t = left / length;
            a = Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
            if on {
                piece.push(a);
                all.push(std::mem::take(&mut piece));
            } else {
                piece = vec![a];
            }
            (length, left, on) = (length - left, dash, !on);
        }
        left -= length;
        if on {
            piece.push(b);
        }
    }
    if on && piece.len() > 1 {
        all.push(piece);
    }
    all
}

/// How far `at` is from a line through `points`.
pub fn distance(points: &[Point], at: Point) -> f32 {
    let to = |a: Point, b: Point| {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let t =
            (((at.x - a.x) * dx + (at.y - a.y) * dy) / (dx * dx + dy * dy).max(f32::EPSILON)).clamp(0.0, 1.0);
        (at.x - a.x - t * dx).hypot(at.y - a.y - t * dy)
    };
    points.windows(2).map(|w| to(w[0], w[1])).fold(f32::INFINITY, f32::min)
}

/// The links as they show: their lines in the box's own pixels, and how each is drawn.
#[derive(Default)]
pub struct Links {
    lines: Vec<(Vec<Point>, LinkStyle)>,
    theme: Theme,
}

impl Links {
    /// Replaces what is drawn; false when it is the same.
    pub fn set(&mut self, lines: Vec<(Vec<Point>, LinkStyle)>) -> bool {
        std::mem::replace(&mut self.lines, lines) != self.lines
    }
}

impl Widget for Links {
    fn is_leaf(&self) -> bool {
        true
    }

    fn measure(&mut self, _cx: &mut MeasureCx, known: Known, _avail: Size) -> Size {
        Size::new(known.width.unwrap_or(0.0), known.height.unwrap_or(0.0))
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        let (scale, origin, clip) = (cx.scale, cx.rect.origin(), cx.clip);
        let device = |p: &Point| Point::new((origin.x + p.x) * scale, (origin.y + p.y) * scale);
        for (points, style) in &self.lines {
            let color = self.theme.resolve(style.color);
            let mut lines = dashes(points, style.dash);
            lines.extend(head(points, style.width).filter(|_| style.arrow).map(|head| head.to_vec()));
            for line in lines {
                let line: Vec<Point> = line.iter().map(device).collect();
                stroke(&line, style.width * scale, color, false, &clip, |instance| cx.push(instance));
            }
        }
    }

    fn theme(&mut self, theme: &Theme) -> bool {
        self.theme = theme.clone();
        true
    }
}

/// The links as a view: a widget over all of the graph's box, whose lines follow the view
/// and the styles.
pub struct LinksView {
    pub view: Signal<GraphView>,
    pub routes: Rc<Vec<(usize, Vec<Point>)>>,
    pub style: Styles,
}

impl View for LinksView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let layout = quadra_layout::Style {
            position: Position::Absolute,
            width: Length::Fill,
            height: Length::Fill,
            ..Default::default()
        };
        let id = cx.tree.insert_widget(Style { layout, ..Style::default() }, Box::new(Links::default()));
        let (writer, LinksView { view, routes, style }) = (cx.handle.clone(), self);
        cx.handle.bind(move || {
            let v = view.get();
            let line = |(k, plane): &(usize, Vec<Point>)| {
                let (k, points) = (*k, plane.iter().map(|&p| v.point(p)).collect());
                let style = style(k);
                (points, LinkStyle { width: style.width * v.zoom.max(0.5), ..style })
            };
            let lines: Vec<_> = routes.iter().map(line).collect();
            writer.with(move |tree| {
                if tree.widget_mut::<Links>(id).is_some_and(|links| links.set(lines)) {
                    tree.mark_paint(id);
                }
            });
        });
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_leaves_and_arrives_at_the_edges_and_a_dashed_one_is_in_pieces() {
        let (a, b) = (Rect::new(0.0, 0.0, 100.0, 40.0), Rect::new(200.0, 100.0, 100.0, 40.0));
        let line = route(a, b, &[]);
        assert_eq!((line[0], *line.last().unwrap()), (Point::new(100.0, 20.0), Point::new(200.0, 120.0)));
        let below = route(a, Rect::new(0.0, 300.0, 100.0, 40.0), &[]);
        assert_eq!((below[0], *below.last().unwrap()), (Point::new(50.0, 40.0), Point::new(50.0, 300.0)));
        let straight = [Point::new(0.0, 0.0), Point::new(100.0, 0.0)];
        let pieces = dashes(&straight, 10.0);
        assert_eq!(pieces.len(), 5);
        assert_eq!((pieces[1][0], pieces[1][1]), (Point::new(20.0, 0.0), Point::new(30.0, 0.0)));
        assert!((distance(&straight, Point::new(50.0, 4.0)) - 4.0).abs() < 1e-4);
        let [wing, tip, _] = head(&straight, 1.0).unwrap();
        assert!(tip == Point::new(100.0, 0.0) && wing.x < 100.0 && wing.y != 0.0);
    }
}
