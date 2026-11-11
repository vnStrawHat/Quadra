//! The paint pass (specs/07, "Display lists"): nodes flagged for paint emit their instances
//! again; the layer takes them in place when every count is unchanged, and is rebuilt by
//! copying clean ranges otherwise.

use quadra_layout::{Display, LayoutTree};
use quadra_prim::{Point, Rect};
use quadra_render::{Clip, Instance, Quad, Rebuild};

use crate::frame::FrameStats;
use crate::node::{NOT_PAINTED, Node, NodeId, flag};
use crate::tree::Tree;
use crate::widget::PaintCx;

/// Overwrite the layer from `at` with the scratch instances `from..to`.
pub(crate) struct Patch {
    at: u32,
    from: u32,
    to: u32,
}

/// The parts of the tree the pass works on, borrowed apart from the scene.
struct Painter<'a> {
    nodes: &'a mut [Node],
    layout: &'a LayoutTree,
    scratch: &'a mut Vec<Instance>,
    patches: &'a mut Vec<Patch>,
    stats: &'a mut FrameStats,
    scale: f32,
    /// Every repainted node emitted as many instances as it had, and no list of children
    /// changed: the layer can be overwritten in place.
    same: bool,
}

/// Paints one node's own instances: the background and border from its style, then its
/// widget. A node that is not displayed paints nothing.
fn paint_own(node: &mut Node, layout: &LayoutTree, rect: Rect, scale: f32, out: &mut Vec<Instance>) {
    let style = layout.style(node.layout);
    if style.display == Display::None {
        return;
    }
    let mut cx = PaintCx { rect, scale, clip: Clip::NONE, out };
    let (background, border_color, radii) =
        (node.paint.background, node.paint.border_color, node.paint.radii);
    cx.quad(Quad {
        bounds: rect,
        background,
        border_widths: style.border,
        border_color,
        radii,
        ..Quad::default()
    });
    if let Some(widget) = &mut node.widget {
        widget.paint(&mut cx);
    }
}

impl Painter<'_> {
    fn hidden(&self, i: usize) -> bool {
        self.layout.style(self.nodes[i].layout).display == Display::None
    }

    /// Walks the flagged paths. Nodes to repaint emit into the scratch buffer; `abs` is the
    /// node's start in the layer as it is now.
    fn collect(&mut self, id: NodeId, origin: Point, abs: u32, force: bool) {
        let i = id.index as usize;
        let flags = self.nodes[i].flags;
        if !force && flags == 0 {
            return;
        }
        let rect = self.layout.rect(self.nodes[i].layout).translate(origin.x, origin.y);
        let force = force || flags & flag::SUBTREE != 0;
        if force || flags & flag::PAINT != 0 {
            let from = self.scratch.len() as u32;
            paint_own(&mut self.nodes[i], self.layout, rect, self.scale, self.scratch);
            let to = self.scratch.len() as u32;
            self.nodes[i].fresh = (from, to);
            self.stats.nodes_painted += 1;
            self.stats.instances_written += to - from;
            self.same &= to - from == self.nodes[i].own;
            self.patches.push(Patch { at: abs, from, to });
        }
        self.same &= flags & flag::RELIST == 0;
        if self.hidden(i) {
            self.same &= self.nodes[i].len == 0;
            return;
        }
        for k in 0..self.nodes[i].children.len() {
            let child = self.nodes[i].children[k];
            let child_abs = abs + self.nodes[child.index as usize].start;
            self.collect(child, rect.origin(), child_abs, force);
        }
    }

    /// After an in-place update: the flags of the walked paths are done.
    fn clear(&mut self, id: NodeId) {
        let i = id.index as usize;
        if self.nodes[i].flags == 0 && self.nodes[i].fresh == NOT_PAINTED {
            return;
        }
        (self.nodes[i].flags, self.nodes[i].fresh) = (0, NOT_PAINTED);
        for k in 0..self.nodes[i].children.len() {
            self.clear(self.nodes[i].children[k]);
        }
    }

    /// Writes the node's subtree into the next list and returns its length there. A clean
    /// subtree is one copy; a repainted node takes its instances from the scratch buffer.
    fn rebuild(&mut self, id: NodeId, old_abs: usize, next: &mut Rebuild) -> u32 {
        let i = id.index as usize;
        let node = &self.nodes[i];
        let (fresh, old_own, old_len) = (node.fresh, node.own as usize, node.len as usize);
        if node.flags == 0 && fresh == NOT_PAINTED {
            next.copy(old_abs..old_abs + old_len);
            return old_len as u32;
        }
        let start = next.len();
        if fresh == NOT_PAINTED {
            next.copy(old_abs..old_abs + old_own);
        } else {
            next.push(&self.scratch[fresh.0 as usize..fresh.1 as usize]);
            self.nodes[i].own = fresh.1 - fresh.0;
        }
        if !self.hidden(i) {
            for k in 0..self.nodes[i].children.len() {
                let child = self.nodes[i].children[k];
                let c = child.index as usize;
                let at = (next.len() - start) as u32;
                // Where the child was: its old start still counts from this node's old start.
                self.nodes[c].len = self.rebuild(child, old_abs + self.nodes[c].start as usize, next);
                self.nodes[c].start = at;
            }
        }
        (self.nodes[i].flags, self.nodes[i].fresh) = (0, NOT_PAINTED);
        (next.len() - start) as u32
    }
}

impl Tree {
    pub(crate) fn paint_pass(&mut self) {
        let Some(root) = self.root.filter(|_| std::mem::take(&mut self.repaint)) else { return };
        self.scratch.clear();
        self.patches.clear();
        let mut painter = Painter {
            nodes: &mut self.nodes,
            layout: &self.layout,
            scratch: &mut self.scratch,
            patches: &mut self.patches,
            stats: &mut self.stats,
            scale: self.scale,
            same: true,
        };
        painter.collect(root, Point::ZERO, 0, false);
        let layer = self.scene.layer_mut(self.root_layer);
        if painter.same {
            for patch in painter.patches.iter().filter(|p| p.to > p.from) {
                layer.patch(patch.at as usize, &painter.scratch[patch.from as usize..patch.to as usize]);
            }
            painter.stats.layers_patched += painter.patches.iter().any(|p| p.to > p.from) as u32;
            painter.clear(root);
        } else {
            let mut next = layer.rebuild();
            let len = painter.rebuild(root, 0, &mut next);
            next.finish();
            painter.nodes[root.index as usize].len = len;
            painter.stats.layers_rebuilt += 1;
        }
    }

    /// Paints the whole tree from scratch and compares it with what the incremental passes
    /// left in the layer, and each node's range with where its instances are. Returns the
    /// first difference, or `None`: the check that incremental paint is exact. Call it after
    /// a frame.
    pub fn verify_paint(&mut self) -> Option<String> {
        let mut fresh = Vec::new();
        let mut wrong = None;
        if let Some(root) = self.root {
            self.verify_node(root, Point::ZERO, 0, &mut fresh, &mut wrong);
        }
        let layer = self.scene.layer(self.root_layer).instances();
        if layer.len() != fresh.len() {
            return Some(format!("the layer has {} instances, from scratch {}", layer.len(), fresh.len()));
        }
        let differs = layer.iter().zip(&fresh).position(|(a, b)| a != b);
        differs.map(|at| format!("instance {at} differs from the one painted from scratch")).or(wrong)
    }

    fn verify_node(
        &mut self,
        id: NodeId,
        origin: Point,
        parent_start: usize,
        out: &mut Vec<Instance>,
        wrong: &mut Option<String>,
    ) {
        let i = id.index as usize;
        let rect = self.layout.rect(self.nodes[i].layout).translate(origin.x, origin.y);
        let start = out.len();
        paint_own(&mut self.nodes[i], &self.layout, rect, self.scale, out);
        let own = out.len() - start;
        if self.layout.style(self.nodes[i].layout).display != Display::None {
            for k in 0..self.nodes[i].children.len() {
                self.verify_node(self.nodes[i].children[k], rect.origin(), start, out, wrong);
            }
        }
        let node = &self.nodes[i];
        let expected = ((start - parent_start) as u32, (out.len() - start) as u32, own as u32);
        if (node.start, node.len, node.own) != expected && wrong.is_none() {
            let have = (node.start, node.len, node.own);
            *wrong =
                Some(format!("node {i}: range (start, len, own) is {have:?}, from scratch {expected:?}"));
        }
    }
}
