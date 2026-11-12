//! What a flowchart paints: the frames of its subgraphs, its links as smooth curves along
//! their nodes that end on the outlines of what they join, the labels of the links, and the
//! shapes of its nodes.

use std::f32::consts::PI;

use quadra_prim::{Point, Rect};

use super::rank::Layers;
use super::{Flow, Shape, Stroke};
use crate::mermaid::curve::{Outline, basis, clip, cut_end, head_len};
use crate::mermaid::draw::{Drawing, Head, Ink, Measure, Op, TextBox};

fn outline(shape: Shape) -> Outline {
    match shape {
        Shape::Diamond => Outline::Diamond,
        Shape::Circle => Outline::Circle,
        _ => Outline::Box,
    }
}

impl Drawing {
    /// Each subgraph as a frame with its title, about its nodes and the frames in it.
    pub fn frames(&mut self, f: &Flow, boxes: &[Rect], m: &mut Measure) {
        let size = m.base.size * 0.85;
        let title = Measure::line(size) + 6.0;
        let mut holds: Vec<Option<Rect>> = vec![None; f.subs.len()];
        let hold = |holds: &mut Vec<Option<Rect>>, sub: usize, rect: Rect| {
            holds[sub] = Some(holds[sub].map_or(rect, |held| held.union(rect)));
        };
        for (v, node) in f.nodes.iter().enumerate() {
            node.sub.into_iter().for_each(|sub| hold(&mut holds, sub, boxes[v]));
        }
        // A subgraph is written after the one it is in: backwards, a frame is known before
        // the frame about it.
        for sub in (0..f.subs.len()).rev() {
            let Some(held) = holds[sub] else { continue };
            let frame = Rect::new(
                held.x - 14.0,
                held.y - 14.0 - title,
                held.width + 28.0,
                held.height + 28.0 + title,
            );
            holds[sub] = Some(frame);
            f.subs[sub].parent.into_iter().for_each(|parent| hold(&mut holds, parent, frame));
        }
        for (sub, frame) in holds.iter().enumerate() {
            let Some(rect) = *frame else { continue };
            self.ops.push(Op::Box { rect, radius: 6.0, fill: Ink::Frame, border: Ink::Line });
            let word = m.word(&f.subs[sub].title, size, 600);
            self.text_left(word, size, rect.x + 10.0, rect.y + 4.0 + title / 2.0, Ink::Faint);
        }
    }

    /// Every link: its curve, its heads and its label.
    pub fn edges(&mut self, f: &Flow, g: &Layers, centers: &[Point], boxes: &[Rect], labels: &[TextBox]) {
        for (k, e) in f.edges.iter().enumerate() {
            let width = if e.stroke == Stroke::Thick { 2.5 } else { 1.5 };
            let chain = &g.chains[k];
            let points = if e.from == e.to {
                // A link from a node to itself: a loop at its right side.
                let (r, cy) = (boxes[e.from].right(), boxes[e.from].center().y);
                let out = |dx: f32, dy: f32| Point::new(r + dx, cy + dy);
                basis(&[out(0.0, -8.0), out(26.0, -14.0), out(26.0, 14.0), out(0.0, 8.0)])
            } else {
                let mut c: Vec<Point> = chain.iter().map(|v| centers[*v]).collect();
                if g.rev[k] {
                    c.reverse();
                }
                let last = c.len() - 1;
                c[0] = clip(boxes[e.from], outline(f.nodes[e.from].shape), c[1]);
                c[last] = clip(boxes[e.to], outline(f.nodes[e.to].shape), c[last - 1]);
                basis(&c)
            };
            let [start, .., tip] = points[..] else { continue };
            let mut line = cut_end(&points, head_len(e.head, width));
            if e.tail != Head::None {
                line.reverse();
                line = cut_end(&line, head_len(e.tail, width));
                line.reverse();
            }
            let (first, middle, last) = (line[0], line[line.len() / 2], line[line.len() - 1]);
            self.line(line, width, e.stroke == Stroke::Dotted, Ink::Accent);
            self.head(tip, last, e.head, Ink::Accent);
            self.head(start, first, e.tail, Ink::Accent);
            let label = &labels[k];
            if !label.lines.is_empty() {
                // On the node in the middle of the link, which was made as large as it.
                let at = if chain.len() > 2 { centers[chain[(chain.len() - 1) / 2]] } else { middle };
                let rect = Rect::new(
                    at.x - label.width / 2.0 - 5.0,
                    at.y - label.height / 2.0 - 2.0,
                    label.width + 10.0,
                    label.height + 4.0,
                );
                self.ops.push(Op::Box { rect, radius: 3.0, fill: Ink::Canvas, border: Ink::None });
                self.text(label, at, Ink::Text);
            }
        }
    }

    /// The outline and the fill of a node in `r`.
    pub fn shape(&mut self, shape: Shape, r: Rect) {
        let (x, y, w, h) = (r.x, r.y, r.width, r.height);
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        let at = Point::new;
        let points = match shape {
            Shape::Rect | Shape::Round | Shape::Stadium | Shape::Circle | Shape::Subroutine => {
                let radius = match shape {
                    Shape::Round => 10.0,
                    Shape::Stadium | Shape::Circle => h / 2.0,
                    Shape::Subroutine => 0.0,
                    _ => 4.0,
                };
                self.ops.push(Op::Box { rect: r, radius, fill: Ink::Panel, border: Ink::Line });
                if shape == Shape::Subroutine {
                    self.line(vec![at(x + 8.0, y), at(x + 8.0, y + h)], 1.0, false, Ink::Line);
                    self.line(vec![at(x + w - 8.0, y), at(x + w - 8.0, y + h)], 1.0, false, Ink::Line);
                }
                return;
            }
            Shape::Diamond => vec![at(cx, y), at(x + w, cy), at(cx, y + h), at(x, cy)],
            Shape::Hexagon => {
                let k = h / 4.0;
                vec![
                    at(x + k, y),
                    at(x + w - k, y),
                    at(x + w, cy),
                    at(x + w - k, y + h),
                    at(x + k, y + h),
                    at(x, cy),
                ]
            }
            Shape::Flag => vec![at(x, y), at(x + w, y), at(x + w, y + h), at(x, y + h), at(x + h / 4.0, cy)],
            Shape::Cylinder => {
                // Half an ellipse at an angle: `side` is the way it goes, `up` the way it bulges.
                let ry = 7.0;
                let arc = |side: f32, middle: f32, up: f32| {
                    let point = move |i: usize| {
                        let a = PI * i as f32 / 16.0;
                        at(cx + side * w / 2.0 * a.cos(), middle + up * ry * a.sin())
                    };
                    (0..17).map(point)
                };
                // The back of the top from right to left, then the front of the bottom back.
                let body = arc(1.0, y + ry, -1.0).chain(arc(-1.0, y + h - ry, 1.0)).collect();
                self.ops.push(Op::Fill { points: body, fill: Ink::Panel, border: Ink::Line });
                self.line(arc(-1.0, y + ry, 1.0).collect(), 1.0, false, Ink::Line);
                return;
            }
        };
        self.ops.push(Op::Fill { points, fill: Ink::Panel, border: Ink::Line });
    }
}
