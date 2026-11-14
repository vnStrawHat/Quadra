//! A frame: the passes, in order, each over what is flagged for it (specs/02, "The frame";
//! specs/04, "Passes"). Style resolution, compose and accessibility join as they are built.

use std::time::{Duration, Instant};

use quadra_layout::{Known, LayoutId, Measure, RootSize};
use quadra_prim::Size;
use quadra_reactive::batch;
use quadra_text::TextSystem;

use crate::inherit::{TextAt, refresh};
use crate::node::{Node, NodeId, flag};
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
    /// Nodes whose style was resolved again because a state changed.
    pub nodes_styled: u32,
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

/// How long the passes of the last frame took. `update` includes the frame callbacks.
#[derive(Clone, Copy, Debug, Default)]
pub struct PassTimes {
    pub update: Duration,
    pub layout: Duration,
    pub paint: Duration,
    pub compose: Duration,
}

/// Gives the layout engine the content size of leaves: it asks their widgets.
struct Measurer<'a> {
    nodes: &'a mut [Node],
    owners: &'a [u32],
    text: &'a mut TextSystem,
    text_at: TextAt,
}

impl Measure for Measurer<'_> {
    fn measure(&mut self, node: LayoutId, known: Known, avail: Size) -> Size {
        let index = self.owners[node.index()] as usize;
        if self.nodes[index].widget.is_none() {
            return Size::ZERO;
        }
        refresh(self.nodes, index, self.text_at);
        let mut cx = MeasureCx { text: self.text };
        self.nodes[index].widget.as_mut().map_or(Size::ZERO, |w| w.measure(&mut cx, known, avail))
    }
}

impl Tree {
    /// True when a frame would do something: nothing flagged, nothing queued means no frame.
    pub fn needs_frame(&self) -> bool {
        self.relayout
            || self.repaint
            || !self.recompose.is_empty()
            || !self.restyle.is_empty()
            || self.shared.wake.get()
            || self.animating()
    }

    /// True while a frame callback is registered, or a change of style is easing: every
    /// frame is wanted.
    pub fn animating(&self) -> bool {
        !self.shared.frames.borrow().is_empty() || !self.moving.is_empty()
    }

    /// Runs the frame callbacks in one batch and keeps those that want the next frame.
    fn run_frame_callbacks(&mut self) {
        let mut callbacks = std::mem::take(&mut *self.shared.frames.borrow_mut());
        if callbacks.is_empty() {
            return;
        }
        batch(|| callbacks.retain_mut(|callback| callback()));
        // Those registered while these ran come after the ones kept.
        let mut registered = self.shared.frames.borrow_mut();
        callbacks.append(&mut registered);
        *registered = callbacks;
    }

    /// Runs the passes for a window of `size` logical pixels at `scale` device pixels each,
    /// and returns what they visited. Afterwards the scene is ready to draw.
    pub fn frame(&mut self, size: Size, scale: f32) -> FrameStats {
        self.stats = FrameStats::default();
        if size != self.size {
            self.relayout = true;
        }
        if scale != self.scale {
            // Snapping and glyph sizes depend on the scale: everything is painted again.
            (self.repaint, self.repaint_all) = (true, true);
        }
        (self.size, self.scale) = (size, scale);
        let (mut times, mut clock) = (PassTimes::default(), Instant::now());
        let mut lap = || std::mem::replace(&mut clock, Instant::now()).elapsed();
        self.run_frame_callbacks();
        self.motion_pass();
        // A layout may change a size that state depends on: its bindings run and the layout
        // is done again. After four rounds the rest waits for the next frame.
        for _ in 0..4 {
            self.update();
            self.style_pass();
            times.update += lap();
            self.layout_pass();
            times.layout += lap();
            if !self.shared.wake.get() {
                break;
            }
        }
        self.place_overlays();
        if self.input.pointer.is_some() {
            // What is under the pointer may have moved away from it, or come under it: a list
            // that scrolled. Its hover shows in this frame; a layout that its handlers ask
            // for waits for the next one.
            batch(|| self.rehover());
            self.update();
            self.style_pass();
            times.update += lap();
        }
        self.cull_pass();
        self.paint_pass();
        times.paint = lap();
        self.compose_pass();
        times.compose = lap();
        (self.times, self.frames, self.snap) = (times, self.frames + 1, false);
        self.stats
    }

    /// How long the passes of the last frame took.
    pub fn times(&self) -> PassTimes {
        self.times
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
            Op::Patch(node, patch) => self.patch_style(node, patch),
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

    /// A node's widget as its own type, to change it; `None` if it is gone or of another
    /// type. Call [`Tree::mark_layout`] or [`Tree::mark_paint`] for what the change needs.
    pub fn widget_mut<T: Widget>(&mut self, id: NodeId) -> Option<&mut T> {
        let widget: &mut dyn std::any::Any = self.get_mut(id)?.widget.as_deref_mut()?;
        widget.downcast_mut()
    }

    /// Paints everything again in the next frame: what was painted is gone with the device
    /// (the glyph tiles), or depends on something every node read.
    pub fn repaint_all(&mut self) {
        (self.repaint, self.repaint_all) = (true, true);
    }

    /// Says that what a leaf measures changed.
    pub fn mark_layout(&mut self, id: NodeId) {
        if let Some(node) = self.get(id) {
            self.layout.mark_dirty(node.layout);
            self.relayout = true;
            self.semantics_changed(id); // what it holds changed: its text, its value
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
        self.semantics_changed(id); // its bounds, and those of what is under it
        let node = &self.nodes[id.index as usize];
        if node.layer.is_some() {
            // A layer that moved keeps its instances: its transform changes.
            if moved {
                self.recompose.push(id);
            }
            if resized {
                self.mark(id, flag::PAINT);
                self.recompose.push(id); // a backdrop is as large as the box
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
        self.lay_out();
        // What asked to see the result may write state: the frame then goes round again.
        for see in std::mem::take(&mut self.after_layout) {
            see(self);
        }
    }

    fn lay_out(&mut self) {
        let Some(root) = self.root.filter(|_| std::mem::take(&mut self.relayout)) else { return };
        let root = self.nodes[root.index as usize].layout;
        let above: Vec<_> = self.overlays().map(|o| self.nodes[o.index as usize].layout).collect();
        {
            let mut fonts = self.fonts.borrow_mut();
            let text_at = self.text_at();
            let mut measurer =
                Measurer { nodes: &mut self.nodes, owners: &self.owners, text: &mut fonts.text, text_at };
            self.layout.layout(root, RootSize::Fill(self.size), &mut measurer);
            // Overlays after the main tree: as large as their content, at most the window.
            for overlay in above {
                self.layout.layout(overlay, RootSize::Fit(self.size), &mut measurer);
            }
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
            if change.resized
                && let Some(&watcher) = self.sizes.get(&index)
            {
                // A watcher whose scope ended is dropped; a slot reused by another node too.
                let size = self.layout.rect(change.id).size();
                if !watcher.try_set(size) {
                    self.sizes.remove(&index);
                }
            }
        }
        self.changes = changes;
    }
}
