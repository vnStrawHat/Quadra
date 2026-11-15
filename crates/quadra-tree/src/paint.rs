//! The paint pass (specs/07, "Display lists"): nodes flagged for paint emit their instances
//! again; their layer takes them in place when every count is unchanged, and is rebuilt by
//! copying clean ranges otherwise. A node that is a layer stands in its parent's list as one
//! marker, and its own content is painted into its own layer.

use quadra_layout::{Display, LayoutTree};
use quadra_prim::{Point, Rect};
use quadra_render::{Instance, Rebuild};

use crate::fonts::Fonts;
use crate::frame::FrameStats;
use crate::inherit::TextAt;
use crate::node::{NOT_PAINTED, Node, NodeId, flag};
pub(crate) use crate::own::{outside, paint_own};
use crate::theme::Theme;
use crate::tree::Tree;

/// Overwrite the layer from `at` with the scratch instances `from..to`.
pub(crate) struct Patch {
    at: u32,
    from: u32,
    to: u32,
}

/// The parts of the tree the pass works on for one layer, borrowed apart from the scene.
struct Painter<'a> {
    nodes: &'a mut [Node],
    layout: &'a LayoutTree,
    scratch: &'a mut Vec<Instance>,
    patches: &'a mut Vec<Patch>,
    stats: &'a mut FrameStats,
    /// Layers found on the way that have something to paint.
    todo: &'a mut Vec<NodeId>,
    /// Layers found on the way whose place changed.
    moved: &'a mut Vec<NodeId>,
    fonts: &'a mut Fonts,
    /// The node whose layer is being painted.
    root: NodeId,
    scale: f32,
    /// What widgets are given their text values with.
    text_at: TextAt,
    /// What the shadows of a level above the window are, and the roles widgets paint with:
    /// the theme of the node being painted.
    theme: &'a Theme,
    /// The nodes that give what is under them a theme of their own.
    themes: &'a [(NodeId, std::rc::Rc<Theme>)],
    /// The part of the layer that is painted, if it paints only near what shows.
    band: Option<Rect>,
    /// The walk is under a node outside the band: nothing is painted.
    culled: bool,
    /// The scale changed: everything is painted, in every layer.
    all: bool,
    /// Every repainted node emitted as many instances as it had, and no list of children
    /// changed: the layer can be overwritten in place.
    same: bool,
}

impl Painter<'_> {
    fn hidden(&self, i: usize) -> bool {
        self.layout.style(self.nodes[i].layout).display == Display::None
    }

    /// A layer other than the one being painted: here it is one marker.
    fn child_layer(&self, id: NodeId) -> bool {
        id != self.root && self.nodes[id.index as usize].layer.is_some()
    }

    /// Walks the flagged paths. Nodes to repaint emit into the scratch buffer; `abs` is the
    /// node's start in the layer as it is now.
    fn collect(&mut self, id: NodeId, origin: Point, abs: u32, force: bool) {
        let outer = self.theme;
        if let Some((_, own)) = self.themes.iter().find(|(own, _)| *own == id) {
            self.theme = own;
        }
        self.collect_in(id, origin, abs, force);
        self.theme = outer;
    }

    fn collect_in(&mut self, id: NodeId, origin: Point, abs: u32, force: bool) {
        let i = id.index as usize;
        let flags = self.nodes[i].flags;
        if !force && flags == 0 {
            return;
        }
        self.stats.nodes_walked += 1;
        if self.child_layer(id) {
            return self.collect_marker(id, abs, force);
        }
        let rect = self.layout.rect(self.nodes[i].layout).translate(origin.x, origin.y);
        // Outside the band nothing is painted, nor anything held: what emitted nothing and
        // has nothing to paint emits nothing again, and its subtree is not walked.
        // ponytail: each child of a long column is still looked at (some 25 ns a row: 0.5 ms
        // a band for 20,000); a search among children placed in order if such pages scroll.
        let culled = self.culled || outside(self.band, rect);
        if culled && flags == 0 && self.nodes[i].len == 0 {
            return;
        }
        let force = force || flags & flag::SUBTREE != 0;
        if force || flags & flag::PAINT != 0 {
            let from = self.scratch.len() as u32;
            if !culled {
                // Under themes of their own, a widget is told the one it paints under.
                if let Some(widget) = self.nodes[i].widget.as_mut().filter(|_| !self.themes.is_empty()) {
                    widget.theme(self.theme);
                }
                let how = (self.scale, self.band, self.text_at, self.theme);
                paint_own(self.nodes, i, self.layout, rect, how, self.fonts, self.scratch);
                self.stats.nodes_painted += 1;
            }
            let to = self.scratch.len() as u32;
            self.nodes[i].fresh = (from, to);
            self.stats.instances_written += to - from;
            self.same &= to - from == self.nodes[i].own;
            self.patches.push(Patch { at: abs, from, to });
        }
        self.same &= flags & flag::RELIST == 0;
        if self.hidden(i) {
            self.same &= self.nodes[i].len == 0;
            return;
        }
        let outer = std::mem::replace(&mut self.culled, culled);
        for k in 0..self.nodes[i].children.len() {
            let child = self.nodes[i].children[k];
            let child_abs = abs + self.nodes[child.index as usize].start;
            self.collect(child, rect.origin(), child_abs, force);
        }
        self.culled = outer;
    }

    /// A child layer met in the walk: here it is one marker, and its own content is left to its
    /// own pass. The marker is written when it appears or goes (a hidden layer has none), and
    /// again whenever what is around it is painted again: that may be another list than before.
    fn collect_marker(&mut self, id: NodeId, abs: u32, force: bool) {
        let i = id.index as usize;
        let shown = !self.hidden(i);
        let Some(state) = self.nodes[i].layer.as_mut() else { return };
        let was = std::mem::replace(&mut state.marked, shown);
        if was != shown || force {
            let from = self.scratch.len() as u32;
            if shown {
                self.scratch.push(Instance::marker(state.id));
            }
            let to = self.scratch.len() as u32;
            self.nodes[i].fresh = (from, to);
            self.same &= was == shown;
            self.patches.push(Patch { at: abs, from, to });
        }
        if shown && (self.all || self.nodes[i].flags != 0) {
            self.todo.push(id);
        }
        if shown && force {
            self.moved.push(id); // something above it moved: its place in this layer changed
        }
    }

    /// After an in-place update: the flags of the walked paths are done.
    fn clear(&mut self, id: NodeId) {
        let i = id.index as usize;
        if self.child_layer(id) {
            self.nodes[i].fresh = NOT_PAINTED; // its flags are for its own pass
            return;
        }
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
        let repainted = fresh != NOT_PAINTED;
        if self.child_layer(id) {
            if repainted {
                next.push(&self.scratch[fresh.0 as usize..fresh.1 as usize]);
                (self.nodes[i].own, self.nodes[i].fresh) = (fresh.1 - fresh.0, NOT_PAINTED);
                return fresh.1 - fresh.0;
            }
            next.copy(old_abs..old_abs + old_len);
            return old_len as u32;
        }
        if node.flags == 0 && !repainted {
            next.copy(old_abs..old_abs + old_len);
            return old_len as u32;
        }
        let start = next.len();
        if repainted {
            next.push(&self.scratch[fresh.0 as usize..fresh.1 as usize]);
            self.nodes[i].own = fresh.1 - fresh.0;
        } else {
            next.copy(old_abs..old_abs + old_own);
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
    /// Paints every layer that has flagged nodes, the root's first: a layer's pass finds the
    /// child layers that have work.
    pub(crate) fn paint_pass(&mut self) {
        let Some(root) = self.root.filter(|_| std::mem::take(&mut self.repaint)) else { return };
        let all = std::mem::take(&mut self.repaint_all);
        let mut todo = std::mem::take(&mut self.todo);
        todo.extend(self.overlays());
        todo.push(root);
        let fonts = self.fonts.clone();
        let mut fonts = fonts.borrow_mut();
        while let Some(layer_root) = todo.pop() {
            self.paint_layer(layer_root, all, &mut todo, &mut fonts);
        }
        drop(fonts);
        self.todo = todo;
    }

    fn paint_layer(&mut self, root: NodeId, all: bool, todo: &mut Vec<NodeId>, fonts: &mut Fonts) {
        let i = root.index as usize;
        // As the root of its own layer the node starts at zero; as a child it is one marker.
        // The two sets of numbers are swapped in for the pass and back out after it.
        let Some(state) = self.nodes[i].layer.as_ref() else { return };
        let (layer_id, len, own, band) = (state.id, state.len, state.own, state.band.filter(|_| state.culls));
        let as_child = (self.nodes[i].start, self.nodes[i].len, self.nodes[i].own);
        (self.nodes[i].start, self.nodes[i].len, self.nodes[i].own) = (0, len, own);
        self.scratch.clear();
        self.patches.clear();
        let origin = self.layout.rect(self.nodes[i].layout).origin();
        let text_at = self.text_at();
        let theme = crate::theme::theme_in(&self.nodes, &self.themes, &self.theme, root);
        let mut painter = Painter {
            nodes: &mut self.nodes,
            layout: &self.layout,
            scratch: &mut self.scratch,
            patches: &mut self.patches,
            stats: &mut self.stats,
            todo,
            moved: &mut self.recompose,
            fonts,
            root,
            scale: self.scale,
            text_at,
            theme,
            themes: &self.themes,
            band,
            culled: false,
            all,
            same: true,
        };
        // The layer's space starts at its root's border box.
        painter.collect(root, Point::new(-origin.x, -origin.y), 0, all);
        let layer = self.scene.layer_mut(layer_id);
        if painter.same {
            let patches = painter.patches.iter().filter(|p| p.to > p.from);
            patches
                .clone()
                .for_each(|p| layer.patch(p.at as usize, &painter.scratch[p.from as usize..p.to as usize]));
            painter.stats.layers_patched += (patches.count() > 0) as u32;
            painter.clear(root);
        } else {
            let mut next = layer.rebuild();
            let len = painter.rebuild(root, 0, &mut next);
            next.finish();
            painter.nodes[i].len = len;
            painter.stats.layers_rebuilt += 1;
        }
        let node = &mut self.nodes[i];
        let own = (node.len, node.own);
        (node.start, node.len, node.own) = as_child;
        if let Some(state) = node.layer.as_mut() {
            (state.len, state.own) = own;
        }
    }
}
