//! The tree: nodes kept across frames, changed where they changed (specs/04).

use std::cell::RefCell;
use std::rc::Rc;

use quadra_layout::{LayoutTree, RectChange};
use quadra_prim::{Point, Rect, Size};
use quadra_render::{Instance, LayerId, Scene};
use quadra_text::TextSystem;

use crate::frame::FrameStats;
use crate::layers::LayerState;
use crate::node::{NOT_PAINTED, Node, NodeId, PaintStyle, Style, flag};
use crate::paint::Patch;
use crate::shared::{Handle, Shared};
use crate::widget::Widget;

pub struct Tree {
    pub(crate) nodes: Vec<Node>,
    pub(crate) free: Vec<u32>,
    pub(crate) layout: LayoutTree,
    /// By layout index: the index of the node that owns that layout node.
    pub(crate) owners: Vec<u32>,
    pub(crate) scene: Scene,
    pub(crate) root_layer: LayerId,
    pub(crate) text: Rc<RefCell<TextSystem>>,
    pub(crate) shared: Rc<Shared>,
    pub(crate) root: Option<NodeId>,
    /// The size and scale of the last frame.
    pub(crate) size: Size,
    pub(crate) scale: f32,
    /// A layout value changed, or the root: the layout pass has work.
    pub(crate) relayout: bool,
    /// Some node has a paint flag.
    pub(crate) repaint: bool,
    /// The scale changed: every layer paints everything.
    pub(crate) repaint_all: bool,
    /// Layers whose constants must be worked out again.
    pub(crate) recompose: Vec<NodeId>,
    /// Layers waiting to be painted, during the paint pass.
    pub(crate) todo: Vec<NodeId>,
    pub(crate) stats: FrameStats,
    // Reused by every pass.
    pub(crate) scratch: Vec<Instance>,
    pub(crate) patches: Vec<Patch>,
    pub(crate) changes: Vec<RectChange>,
}

impl Tree {
    /// A tree whose text is shaped by `text`, which every window of an app shares.
    pub fn new(text: Rc<RefCell<TextSystem>>) -> Tree {
        let mut scene = Scene::new();
        let root_layer = scene.add_layer();
        scene.set_roots(&[root_layer]);
        Tree {
            nodes: Vec::new(),
            free: Vec::new(),
            layout: LayoutTree::new(),
            owners: Vec::new(),
            scene,
            root_layer,
            text,
            shared: Rc::default(),
            root: None,
            size: Size::ZERO,
            scale: 1.0,
            relayout: false,
            repaint: false,
            repaint_all: false,
            recompose: Vec::new(),
            todo: Vec::new(),
            stats: FrameStats::default(),
            scratch: Vec::new(),
            patches: Vec::new(),
            changes: Vec::new(),
        }
    }

    /// What closures hold to reach this tree.
    pub fn handle(&self) -> Handle {
        Handle(self.shared.clone())
    }

    /// The layers this tree paints into: what a renderer draws.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    /// Adds a box: a node that is only its style and its children.
    pub fn insert(&mut self, style: Style) -> NodeId {
        self.add(style, None)
    }

    /// Adds a node with a widget.
    pub fn insert_widget(&mut self, style: Style, widget: Box<dyn Widget>) -> NodeId {
        self.add(style, Some(widget))
    }

    fn add(&mut self, style: Style, widget: Option<Box<dyn Widget>>) -> NodeId {
        let leaf = widget.as_ref().is_some_and(|w| w.is_leaf());
        let layout =
            if leaf { self.layout.insert_leaf(style.layout) } else { self.layout.insert(style.layout) };
        let index = self.free.pop().unwrap_or(self.nodes.len() as u32);
        let generation = self.nodes.get(index as usize).map_or(0, |n| n.generation);
        let node = Node {
            generation,
            alive: true,
            parent: None,
            children: Vec::new(),
            layout,
            // New: painted with everything under it once it is in the tree.
            flags: flag::PAINT | flag::SUBTREE,
            paint: style.paint,
            widget,
            start: 0,
            len: 0,
            own: 0,
            fresh: NOT_PAINTED,
            layer: None,
            scope: None,
        };
        if index as usize == self.nodes.len() {
            self.nodes.push(node);
        } else {
            self.nodes[index as usize] = node;
        }
        if self.owners.len() <= layout.index() {
            self.owners.resize(layout.index() + 1, 0);
        }
        self.owners[layout.index()] = index;
        NodeId { index, generation }
    }

    /// The node of an id, or `None` when it was removed.
    pub(crate) fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.index as usize).filter(|n| n.alive && n.generation == id.generation)
    }

    pub(crate) fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id.index as usize).filter(|n| n.alive && n.generation == id.generation)
    }

    pub(crate) fn node(&self, id: NodeId) -> &Node {
        self.get(id).expect("a node was used after it was removed")
    }

    pub fn is_alive(&self, id: NodeId) -> bool {
        self.get(id).is_some()
    }

    /// The number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len() - self.free.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    /// Makes `id` the node that fills the window.
    pub fn set_root(&mut self, id: NodeId) {
        self.node(id);
        if self.root == Some(id) {
            return;
        }
        // The root is the layer the scene draws first; a node that was the root is one no more.
        if let Some(old) = self.root.filter(|&old| self.is_alive(old)) {
            self.nodes[old.index as usize].layer = None;
        }
        if let Some(state) = self.nodes[id.index as usize].layer.take() {
            self.scene.remove_layer(state.id);
        }
        self.nodes[id.index as usize].layer = Some(Box::new(LayerState::new(self.root_layer)));
        self.scene.layer_mut(self.root_layer).set(&[]);
        self.root = Some(id);
        self.relayout = true;
        self.mark(id, flag::PAINT | flag::SUBTREE | flag::RELIST);
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).parent
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        &self.node(id).children
    }

    /// Sets paint flags on a node and tells its ancestors a descendant has some.
    pub(crate) fn mark(&mut self, id: NodeId, bits: u8) {
        self.repaint = true;
        self.nodes[id.index as usize].flags |= bits;
        let mut up = self.nodes[id.index as usize].parent;
        while let Some(parent) = up {
            let node = &mut self.nodes[parent.index as usize];
            if node.flags & flag::CHILD != 0 {
                break;
            }
            node.flags |= flag::CHILD;
            up = node.parent;
        }
    }

    /// The node's border box, relative to its parent's.
    pub fn rect(&self, id: NodeId) -> Rect {
        self.layout.rect(self.node(id).layout)
    }

    /// The node's border box in the window, in logical pixels.
    pub fn window_rect(&self, id: NodeId) -> Rect {
        let mut origin = Point::ZERO;
        let mut up = self.node(id).parent;
        while let Some(parent) = up {
            let r = self.layout.rect(self.nodes[parent.index as usize].layout);
            origin = Point::new(origin.x + r.x, origin.y + r.y);
            up = self.nodes[parent.index as usize].parent;
        }
        self.rect(id).translate(origin.x, origin.y)
    }

    pub fn layout_style(&self, id: NodeId) -> &quadra_layout::Style {
        self.layout.style(self.node(id).layout)
    }

    pub fn paint_style(&self, id: NodeId) -> PaintStyle {
        self.node(id).paint
    }
}
