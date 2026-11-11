//! A frame: the passes, in order, each over what is flagged for it (specs/02, "The frame";
//! specs/04, "Passes"). Style resolution, compose and accessibility join as they are built.

use quadra_layout::{Known, LayoutId, Measure, RootSize};
use quadra_prim::Size;
use quadra_text::TextSystem;

use crate::node::{Node, NodeId, PaintStyle, flag};
use crate::shared::Op;
use crate::text::Text;
use crate::tree::Tree;
use crate::widget::{MeasureCx, Widget};

/// What a frame visited. Tests assert on these: the structural budgets of specs/14.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameStats {
    /// Bindings that were due and were brought up to date.
    pub bindings_run: u32,
    /// Property writes applied.
    pub writes: u32,
    /// Requests a layout node had to work out.
    pub nodes_laid_out: u32,
    /// Leaves measured.
    pub nodes_measured: u32,
    /// Nodes whose paint ran.
    pub nodes_painted: u32,
    pub instances_written: u32,
    /// Layers whose instances were overwritten in place.
    pub layers_patched: u32,
    /// Layers whose display list was rebuilt.
    pub layers_rebuilt: u32,
    /// Layers whose transform, cut or opacity changed.
    pub layers_composed: u32,
}

/// Gives the layout engine the content size of leaves: it asks their widgets.
struct Measurer<'a> {
    nodes: &'a mut [Node],
    owners: &'a [u32],
    text: &'a mut TextSystem,
}

impl Measure for Measurer<'_> {
    fn measure(&mut self, node: LayoutId, known: Known, avail: Size) -> Size {
        let widget = &mut self.nodes[self.owners[node.index()] as usize].widget;
        widget.as_mut().map_or(Size::ZERO, |w| w.measure(&mut MeasureCx { text: self.text }, known, avail))
    }
}

impl Tree {
    /// True when a frame would do something: nothing flagged, nothing queued means no frame.
    pub fn needs_frame(&self) -> bool {
        self.relayout || self.repaint || !self.recompose.is_empty() || self.shared.wake.get()
    }

    /// Runs the passes for a window of `size` logical pixels at `scale` device pixels each,
    /// and returns what they visited. Afterwards the scene is ready to draw.
    pub fn frame(&mut self, size: Size, scale: f32) -> FrameStats {
        self.stats = FrameStats::default();
        self.update();
        if size != self.size {
            self.relayout = true;
        }
        if scale != self.scale {
            // Snapping and glyph sizes depend on the scale: everything is painted again.
            (self.repaint, self.repaint_all) = (true, true);
        }
        (self.size, self.scale) = (size, scale);
        self.layout_pass();
        self.paint_pass();
        self.compose_pass();
        self.stats
    }

    /// The update pass: bindings that are due run and queue their writes; the writes are
    /// applied. Applying one may mount nodes, which queues more: repeat until none is left.
    fn update(&mut self) {
        self.shared.wake.set(false);
        loop {
            let due = std::mem::take(&mut *self.shared.due.borrow_mut());
            let ops = std::mem::take(&mut *self.shared.ops.borrow_mut());
            if due.is_empty() && ops.is_empty() {
                break;
            }
            self.stats.bindings_run += due.len() as u32;
            due.into_iter().for_each(|binding| binding.run());
            self.stats.writes += ops.len() as u32;
            ops.into_iter().for_each(|op| self.apply(op));
        }
        self.shared.wake.set(false);
    }

    fn apply(&mut self, op: Op) {
        match op {
            Op::Text(node, text) => self.set_text(node, &text),
            Op::Layout(node, style) => self.set_layout_style(node, *style),
            Op::Paint(node, style) => self.set_paint_style(node, style),
            Op::With(f) => f(self),
        }
    }

    /// Sets the text of a text node. Nothing happens if it is equal, or the node is gone or
    /// is not text.
    pub fn set_text(&mut self, id: NodeId, text: &str) {
        if self.widget_mut::<Text>(id).is_some_and(|t| t.set_text(text)) {
            self.mark_layout(id);
            self.mark(id, flag::PAINT);
        }
    }

    /// Replaces a node's layout style. Nothing happens if it is equal or the node is gone.
    pub fn set_layout_style(&mut self, id: NodeId, style: quadra_layout::Style) {
        let Some(node) = self.get(id) else { return };
        let layout = node.layout;
        let old = self.layout.style(layout);
        if *old != style {
            // Border widths and the frame the content sits in are painted from this style.
            // Showing or hiding changes how many instances the subtree has: ranges move.
            let shown = if old.display != style.display { flag::RELIST } else { 0 };
            self.layout.set_style(layout, style);
            self.relayout = true;
            self.mark(id, flag::PAINT | flag::SUBTREE | shown);
        }
    }

    /// Replaces a node's paint style. Nothing happens if it is equal or the node is gone.
    pub fn set_paint_style(&mut self, id: NodeId, style: PaintStyle) {
        let Some(node) = self.get_mut(id) else { return };
        if node.paint != style {
            node.paint = style;
            self.mark(id, flag::PAINT);
        }
    }

    /// A node's widget as its own type, to change it; `None` if it is gone or of another
    /// type. Call [`Tree::mark_layout`] or [`Tree::mark_paint`] for what the change needs.
    pub fn widget_mut<T: Widget>(&mut self, id: NodeId) -> Option<&mut T> {
        let widget: &mut dyn std::any::Any = self.get_mut(id)?.widget.as_deref_mut()?;
        widget.downcast_mut()
    }

    /// Says that what a leaf measures changed.
    pub fn mark_layout(&mut self, id: NodeId) {
        if let Some(node) = self.get(id) {
            self.layout.mark_dirty(node.layout);
            self.relayout = true;
        }
    }

    /// Says that what a node paints changed.
    pub fn mark_paint(&mut self, id: NodeId) {
        if self.get(id).is_some() {
            self.mark(id, flag::PAINT);
        }
    }

    /// Flags a node whose rect a layout changed.
    fn rect_changed(&mut self, id: NodeId, moved: bool, resized: bool) {
        let node = &self.nodes[id.index as usize];
        if node.layer.is_some() {
            // A layer that moved keeps its instances: its transform changes.
            if moved {
                self.recompose.push(id);
            }
            if resized {
                self.mark(id, flag::PAINT);
            }
        } else {
            // Positions in a layer are absolute: a node that moved takes its subtree along.
            self.mark(id, if moved { flag::PAINT | flag::SUBTREE } else { flag::PAINT });
        }
        if resized {
            // A layer cut to this node's box is cut anew.
            for k in 0..self.nodes[id.index as usize].children.len() {
                let child = self.nodes[id.index as usize].children[k];
                if self.nodes[child.index as usize].layer.as_ref().is_some_and(|s| s.clip_to_parent) {
                    self.recompose.push(child);
                }
            }
        }
    }

    /// The layout pass: lays out what changed, then flags for paint the nodes whose rect did.
    fn layout_pass(&mut self) {
        let Some(root) = self.root.filter(|_| std::mem::take(&mut self.relayout)) else { return };
        let root = self.nodes[root.index as usize].layout;
        {
            let mut text = self.text.borrow_mut();
            let mut measurer = Measurer { nodes: &mut self.nodes, owners: &self.owners, text: &mut text };
            self.layout.layout(root, RootSize::Fill(self.size), &mut measurer);
        }
        let stats = self.layout.take_stats();
        self.stats.nodes_laid_out += stats.asked;
        self.stats.nodes_measured += stats.measured;
        let mut changes = std::mem::take(&mut self.changes);
        self.layout.take_changes(&mut changes);
        for change in &changes {
            let index = self.owners[change.id.index()];
            let id = NodeId { index, generation: self.nodes[index as usize].generation };
            self.rect_changed(id, change.moved, change.resized);
        }
        self.changes = changes;
    }
}
