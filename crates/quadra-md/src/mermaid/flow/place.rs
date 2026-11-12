//! Where the nodes of a flowchart are: their sizes from their text and shape, their places
//! across the ranks by going to the median of what they are linked to again and again (a
//! rank is spread out again each time, its order kept), and along the ranks by how thick
//! each rank is.

use quadra_prim::{Point, Rect};

use super::rank::{LNode, Layers, common, layers};
use super::{Dir, Flow, Shape};
use crate::mermaid::draw::{Drawing, Ink, Measure, TextBox};

/// The room a frame takes on each side, and what a title adds to its height.
const FRAME: f32 = 18.0;

fn node_size(shape: Shape, text: &TextBox) -> (f32, f32) {
    let (w, h) = (text.width + 32.0, (text.height + 20.0).max(38.0));
    let round = text.width.max(text.height) + 28.0;
    let square = text.width + text.height + 30.0;
    match shape {
        Shape::Rect | Shape::Round => (w, h),
        Shape::Stadium => (text.width + h + 12.0, h),
        Shape::Circle => (round, round),
        Shape::Diamond => (square, square),
        Shape::Hexagon => (w + h / 2.0, h),
        Shape::Subroutine => (w + 16.0, h),
        Shape::Cylinder => (w, h + 14.0),
        Shape::Flag => (w + h / 4.0, h),
    }
}

fn median(mut values: Vec<f32>) -> f32 {
    values.sort_by(f32::total_cmp);
    let n = values.len();
    if n % 2 == 1 { values[n / 2] } else { (values[n / 2 - 1] + values[n / 2]) / 2.0 }
}

impl Layers {
    /// How far apart the middles of two neighbours of a rank are.
    fn sep(&self, a: usize, b: usize, vertical: bool) -> f32 {
        let (na, nb) = (&self.n[a], &self.n[b]);
        let gap = match (na.real, nb.real) {
            (true, true) if vertical => 40.0,
            (true, true) => 26.0,
            (false, false) => 12.0,
            _ => 18.0,
        };
        // Between them are the sides of the frames only one of them is in; across the ranks
        // of a chart that goes sideways, their titles too.
        let frames = na.path.len() + nb.path.len() - 2 * common(&na.path, &nb.path);
        (na.w + nb.w) / 2.0 + gap + if vertical { FRAME } else { 40.0 } * frames as f32
    }

    /// Puts the nodes of a rank as near to where they want to be as their order and what is
    /// between them allow: the mean of them packed from the left and packed from the right,
    /// both of which hold, so it does.
    fn spread(&mut self, r: usize, want: &[f32], vertical: bool) {
        let row = &self.rows[r];
        let (mut left, mut right) = (want.to_vec(), want.to_vec());
        for i in 1..row.len() {
            left[i] = want[i].max(left[i - 1] + self.sep(row[i - 1], row[i], vertical));
        }
        for i in (1..row.len()).rev() {
            right[i - 1] = want[i - 1].min(right[i] - self.sep(row[i - 1], row[i], vertical));
        }
        for (i, v) in row.iter().enumerate() {
            self.n[*v].pos = (left[i] + right[i]) / 2.0;
        }
    }

    fn place(&mut self, vertical: bool) {
        for r in 0..self.rows.len() {
            let mut at = 0.0;
            for i in 0..self.rows[r].len() {
                at += if i > 0 { self.sep(self.rows[r][i - 1], self.rows[r][i], vertical) } else { 0.0 };
                let v = self.rows[r][i];
                self.n[v].pos = at;
            }
        }
        for turn in 0..12 {
            // Towards the rank above, towards the one below (from the last rank up), both.
            let mode = turn % 3;
            for k in 0..self.rows.len() {
                let r = if mode == 1 { self.rows.len() - 1 - k } else { k };
                let want = |v: &usize| {
                    let node = &self.n[*v];
                    let (up, down) =
                        (node.up.iter().filter(|_| mode != 1), node.down.iter().filter(|_| mode != 0));
                    let near: Vec<f32> = up.chain(down).map(|w| self.n[*w].pos).collect();
                    if near.is_empty() { node.pos } else { median(near) }
                };
                let want: Vec<f32> = self.rows[r].iter().map(want).collect();
                self.spread(r, &want, vertical);
            }
        }
        let low = self.n.iter().map(|node| node.pos - node.w / 2.0).fold(f32::MAX, f32::min);
        self.n.iter_mut().for_each(|node| node.pos -= low);
    }
}

/// The room along the ranks before and after each rank for the frames of the subgraphs that
/// start or end there: their sides and, in a chart that goes down, their titles.
fn frame_room(f: &Flow, g: &Layers, title: f32) -> (Vec<f32>, Vec<f32>) {
    let (mut before, mut after) = (vec![0.0; g.rows.len()], vec![0.0; g.rows.len()]);
    let mut spans: Vec<Option<(usize, usize)>> = vec![None; f.subs.len()];
    for node in g.n.iter().filter(|node| node.real) {
        for sub in &node.path {
            let (low, high) = spans[*sub].unwrap_or((node.rank, node.rank));
            spans[*sub] = Some((low.min(node.rank), high.max(node.rank)));
        }
    }
    let backwards = matches!(f.dir(), Dir::Up | Dir::Left);
    for (low, high) in spans.into_iter().flatten() {
        let (top, bottom) = if backwards { (high, low) } else { (low, high) };
        before[top] += FRAME + if f.dir().vertical() { title } else { 0.0 };
        after[bottom] += FRAME;
    }
    // Where the ranks run backwards, what is before one is after it.
    if backwards { (after, before) } else { (before, after) }
}

/// Lays a flowchart out: what to paint.
pub fn layout(f: &Flow, m: &mut Measure) -> Drawing {
    let size = m.base.size;
    let texts: Vec<TextBox> = f.nodes.iter().map(|node| m.text(&node.label, size, 400)).collect();
    let labels: Vec<TextBox> = f.edges.iter().map(|edge| m.text(&edge.label, size * 0.92, 400)).collect();
    let (dir, vertical) = (f.dir(), f.dir().vertical());
    let across = |w: f32, h: f32| if vertical { (w, h) } else { (h, w) };
    let mut g = layers(f);
    // The label of a long link has the node in its middle to itself.
    for (k, chain) in
        g.chains.iter().enumerate().filter(|(k, chain)| !labels[*k].lines.is_empty() && chain.len() > 2)
    {
        let middle = &mut g.n[chain[(chain.len() - 1) / 2]];
        (middle.w, middle.h) = across(labels[k].width + 10.0, labels[k].height + 4.0);
    }
    let sizes: Vec<(f32, f32)> =
        f.nodes.iter().zip(&texts).map(|(node, text)| node_size(node.shape, text)).collect();
    for (v, (w, h)) in sizes.iter().enumerate() {
        (g.n[v].w, g.n[v].h) = across(*w, *h);
    }
    g.order();
    g.place(vertical);
    // Along the ranks: a rank is as thick as its thickest node, with half a gap after it.
    let half = if vertical { 26.0 } else { 30.0 };
    let (before, after) = frame_room(f, &g, Measure::line(size * 0.85) + 6.0);
    let (mut middles, mut at) = (Vec::with_capacity(g.rows.len()), 0.0f32);
    for (r, row) in g.rows.iter().enumerate() {
        let thick = row.iter().map(|v| g.n[*v].h).fold(0.0, f32::max);
        middles.push(at + before[r] + thick / 2.0);
        at += before[r] + thick + half + after[r];
    }
    let total = at - half;
    let center = |node: &LNode| {
        let along = middles[node.rank];
        match dir {
            Dir::Down => Point::new(node.pos, along),
            Dir::Up => Point::new(node.pos, total - along),
            Dir::Right => Point::new(along, node.pos),
            Dir::Left => Point::new(total - along, node.pos),
        }
    };
    let centers: Vec<Point> = g.n.iter().map(center).collect();
    let rect = |(c, (w, h)): (&Point, &(f32, f32))| Rect::new(c.x - w / 2.0, c.y - h / 2.0, *w, *h);
    let boxes: Vec<Rect> = centers.iter().zip(&sizes).map(rect).collect();
    let mut d = Drawing::default();
    d.frames(f, &boxes, m);
    d.edges(f, &g, &centers, &boxes, &labels);
    for (v, node) in f.nodes.iter().enumerate() {
        d.shape(node.shape, boxes[v]);
        // The text of a cylinder is in its body, under its rim.
        let down = if node.shape == Shape::Cylinder { 5.0 } else { 0.0 };
        let middle = boxes[v].center();
        d.text(&texts[v], Point::new(middle.x, middle.y + down), Ink::Text);
    }
    d.normalize(8.0);
    d
}
