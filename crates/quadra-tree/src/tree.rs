//! The tree: nodes kept across frames, changed where they changed (specs/04).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use quadra_layout::{LayoutTree, RectChange};
use quadra_prim::{Point, Rect, Size};
use quadra_reactive::{Signal, signal};
use quadra_render::{Instance, LayerId, Scene};

use crate::action::Keys;
use crate::fonts::Fonts;
use crate::frame::{FrameStats, PassTimes};
use crate::input::InputState;
use crate::layers::LayerState;
use crate::node::{NOT_PAINTED, Node, NodeId, PaintStyle, Style, flag};
use crate::overlay::Overlay;
use crate::paint::Patch;
use crate::shared::{Handle, Shared};
use crate::style::State;
use crate::widget::Widget;

pub struct Tree {
    pub(crate) nodes: Vec<Node>,
    pub(crate) free: Vec<u32>,
    pub(crate) layout: LayoutTree,
    /// By layout index: the index of the node that owns that layout node.
    pub(crate) owners: Vec<u32>,
    pub(crate) scene: Scene,
    pub(crate) root_layer: LayerId,
    pub(crate) fonts: Rc<RefCell<Fonts>>,
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
    /// The layers that paint only near what shows.
    pub(crate) culled: Vec<NodeId>,
    /// The open overlays, the lowest first: roots above the main tree.
    pub(crate) overlays: Vec<Overlay>,
    /// The pointer: where it is, what it is over, what it pressed.
    pub(crate) input: InputState,
    /// Nodes whose state changed and that have a style variant for it.
    pub(crate) restyle: Vec<NodeId>,
    /// The node keys go to, and whether the window itself has the keyboard.
    pub(crate) focus: Option<NodeId>,
    pub(crate) active: bool,
    /// The focus came by the keyboard: it shows its ring.
    pub(crate) focus_ring: bool,
    /// The keymap, and the keys of a sequence typed so far.
    pub(crate) keys: Keys,
    /// Layers waiting to be painted, during the paint pass.
    pub(crate) todo: Vec<NodeId>,
    /// By node index: the signal that follows the node's size (`Tree::size_signal`).
    pub(crate) sizes: HashMap<u32, Signal<Size>>,
    pub(crate) stats: FrameStats,
    pub(crate) times: PassTimes,
    // Reused by every pass.
    pub(crate) scratch: Vec<Instance>,
    pub(crate) patches: Vec<Patch>,
    pub(crate) changes: Vec<RectChange>,
}

impl Tree {
    /// A tree whose text is shaped and painted with `fonts`, which every window of an app
    /// shares.
    pub fn new(fonts: Rc<RefCell<Fonts>>) -> Tree {
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
            fonts,
            shared: Rc::default(),
            root: None,
            size: Size::ZERO,
            scale: 1.0,
            relayout: false,
            repaint: false,
            repaint_all: false,
            recompose: Vec::new(),
            culled: Vec::new(),
            overlays: Vec::new(),
            input: InputState::default(),
            restyle: Vec::new(),
            focus: None,
            active: true,
            focus_ring: false,
            keys: Keys::default(),
            todo: Vec::new(),
            sizes: HashMap::new(),
            stats: FrameStats::default(),
            times: PassTimes::default(),
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
            handlers: Vec::new(),
            focusable: false,
            focus_scope: false,
            key_context: None,
            state: State::NONE,
            styled: None,
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

    /// A signal of the node's size, for state that depends on it (how many rows of a list
    /// fit). It is written after the layout pass, and the frame then runs its bindings and
    /// lays out again: one more round, not a loop inside the pass (specs/04).
    pub fn size_signal(&mut self, id: NodeId) -> Signal<Size> {
        let size = self.rect(id).size();
        *self.sizes.entry(id.index).or_insert_with(|| signal(size))
    }

    /// The node's border box, relative to its parent's.
    pub fn rect(&self, id: NodeId) -> Rect {
        self.layout.rect(self.node(id).layout)
    }

    /// The node's border box in the window, in logical pixels: where it is drawn, with the
    /// offsets of the layers around it (scrolling) counted in.
    pub fn window_rect(&self, id: NodeId) -> Rect {
        let mut origin = Point::ZERO;
        let mut up = Some(id);
        while let Some(at) = up {
            let node = &self.nodes[at.index as usize];
            let offset = node.layer.as_ref().map_or(Point::ZERO, |layer| layer.offset);
            let place = if at == id { Point::ZERO } else { self.layout.rect(node.layout).origin() };
            origin = Point::new(origin.x + place.x + offset.x, origin.y + place.y + offset.y);
            up = node.parent;
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
