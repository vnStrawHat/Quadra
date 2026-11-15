//! The tree: nodes kept across frames, changed where they changed (specs/04).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use quadra_layout::{LayoutTree, RectChange};
use quadra_prim::Size;
use quadra_reactive::Signal;
use quadra_render::{Instance, LayerId, Scene};

use crate::action::Keys;
use crate::fonts::Fonts;
use crate::frame::{FrameStats, PassTimes};
use crate::input::InputState;
use crate::layers::LayerState;
use crate::node::{NOT_PAINTED, Node, NodeId, Style, flag};
use crate::overlay::Overlay;
use crate::paint::Patch;
use crate::semantics::Listening;
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
    /// How much larger the whole UI is drawn than the window's own scale (specs/12, "Text size").
    pub(crate) zoom: f32,
    pub(crate) text_scale: f32,
    /// The window's own size and scale, before the zoom.
    pub(crate) window: (Size, f32),
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
    /// Who listens to the semantic tree, and what they have not heard yet.
    pub(crate) listening: Listening,
    /// Layers waiting to be painted, during the paint pass.
    pub(crate) todo: Vec<NodeId>,
    /// By node index: the signal that follows the node's size (`Tree::size_signal`).
    pub(crate) sizes: HashMap<u32, Signal<Size>>,
    /// Whether the window is maximized, once something asked.
    pub(crate) maximized: Option<Signal<bool>>,
    /// Counts the changes to what text values hold where: a node's own, or its place.
    pub(crate) text_epoch: u32,
    /// What gives the roles in styles their colours.
    pub(crate) theme: Rc<crate::theme::Theme>,
    /// The nodes that give what is under them a theme of their own.
    pub(crate) themes: Vec<(NodeId, Rc<crate::theme::Theme>)>,
    /// The press that may become a drag, or the drag it became.
    pub(crate) drag: Option<crate::drag::Drag>,
    /// What waits to see the next layout.
    pub(crate) after_layout: Vec<crate::shared::OnTree>,
    /// The nodes with a change that eases under way; the frames so far; and whether what
    /// is styled in this one shows at once (a new theme).
    pub(crate) moving: Vec<NodeId>,
    pub(crate) frames: u64,
    pub(crate) snap: bool,
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
            zoom: 1.0,
            text_scale: 1.0,
            window: (Size::ZERO, 1.0),
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
            listening: Listening::default(),
            todo: Vec::new(),
            sizes: HashMap::new(),
            maximized: None,
            text_epoch: 1,
            theme: Rc::default(),
            themes: Vec::new(),
            drag: None,
            after_layout: Vec::new(),
            moving: Vec::new(),
            frames: 0,
            snap: false,
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

    fn add(&mut self, style: Style, mut widget: Option<Box<dyn Widget>>) -> NodeId {
        widget.iter_mut().for_each(|widget| _ = widget.theme(&self.theme));
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
            text: style.text.is_set().then(|| Box::new(style.text)),
            inherited_at: 0,
            widget,
            start: 0,
            len: 0,
            own: 0,
            fresh: NOT_PAINTED,
            layer: None,
            scope: None,
            handlers: Vec::new(),
            focusable: false,
            tab_index: 0,
            cursor: None,
            region: Default::default(),
            focus_scope: false,
            key_context: None,
            semantics: None,
            state: State::NONE,
            styled: None,
            motion: None,
            fill: None,
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
        let id = NodeId { index, generation };
        self.put_pose(id, style.pose);
        id
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
}
