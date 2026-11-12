//! Pointer input (specs/09): the event a handler gets, handlers on nodes, and hit testing over
//! the tree as it was laid out. Nothing is prepared per frame for it.

use std::any::{Any, TypeId};
use std::rc::Rc;
use std::time::Instant;

use quadra_a11y::SemanticAction;
use quadra_layout::{Display, Overflow};
use quadra_platform::{Cursor, Mods, MouseButton};
use quadra_prim::{Point, Rect, Size};

use crate::focus::KeyEvent;
use crate::node::NodeId;
use crate::tree::Tree;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerKind {
    Down,
    Up,
    Move,
    /// The pointer came over the node or something under it. Not dispatched along a path:
    /// each node that is entered gets its own.
    Enter,
    Leave,
    /// A press and its release on the same node, or on nodes under it.
    Click,
    /// The wheel turned, or two fingers moved: `delta` says how far to scroll.
    Wheel,
}

/// What a pointer handler is given: the event, and the controls of its dispatch.
#[derive(Clone, Debug)]
pub struct PointerEvent {
    pub kind: PointerKind,
    pub button: MouseButton,
    /// In the window, in logical pixels.
    pub at: Point,
    /// From the top-left corner of the node the handler is on.
    pub local: Point,
    /// How large the node the handler is on is: `local` over it is how far across the
    /// pointer is.
    pub size: Size,
    pub mods: Mods,
    /// For a click: 1, or 2 for a double click, and so on.
    pub clicks: u8,
    /// For the wheel: how far to scroll, in pixels. Positive `y` is towards the start of the
    /// content. A handler that scrolls takes what it used off it; what is left goes on up.
    pub delta: Point,
    /// The node the event is for: the deepest one under the pointer, or the one that captured.
    pub target: NodeId,
    /// The node the handler is on.
    pub node: NodeId,
    pub(crate) stopped: bool,
    pub(crate) captured: Option<NodeId>,
}

impl PointerEvent {
    /// Ends the dispatch: no handler after this one runs.
    pub fn stop(&mut self) {
        self.stopped = true;
    }

    /// In a handler of a press: moves and the release go to this node wherever the pointer
    /// is, until the release.
    pub fn capture_pointer(&mut self) {
        self.captured = Some(self.node);
    }
}

type ActionFn = Box<dyn FnMut(&dyn Any)>;
type CompositionFn = Box<dyn FnMut(&str, usize)>;
/// What is told of a press outside an overlay: the nodes the press is on.
type OutsideFn = Box<dyn FnMut(&[NodeId])>;

/// What takes a dropped value: the value, and where in the window it was let go.
pub(crate) type DropFn = Box<dyn FnMut(&dyn Any, Point)>;

/// What a node does when events reach it. `capture`: on the way down from the root, before
/// the handlers on the way up.
pub(crate) enum Handler {
    Pointer {
        kind: PointerKind,
        capture: bool,
        run: Box<dyn FnMut(&mut PointerEvent)>,
    },
    Key {
        down: bool,
        capture: bool,
        run: Box<dyn FnMut(&mut KeyEvent)>,
    },
    /// Focus came to the node (true) or left it.
    Focus(Box<dyn FnMut(bool)>),
    /// Text was typed while the node had the focus.
    Text(Box<dyn FnMut(&str)>),
    /// An input method is composing text for the node: the text so far and the caret in it.
    Composition(CompositionFn),
    /// A press landed outside the overlay the node is the root of.
    /// It is given the path of the press, the deepest node first.
    OutsidePress(OutsideFn),
    /// The node can be dragged: asked for the value when a drag of it starts. `off`, when
    /// not zero, is how far above or below the press the pointer goes before that.
    Drag {
        off: f32,
        start: Box<dyn FnMut() -> Rc<dyn Any>>,
    },
    /// The node follows the pointer while what it takes is dragged over it: where the
    /// pointer is in the window, and `None` when it leaves or the drag ends.
    DragOver {
        of: TypeId,
        run: Box<dyn FnMut(Option<Point>)>,
    },
    /// The node takes what is dragged when it is of a type: the value, and where it was let
    /// go in the window.
    Drop {
        of: TypeId,
        run: DropFn,
    },
    /// The node scrolls what it holds: asked to show a rect of its own box, it says how far
    /// down it scrolled for it.
    Reveal(Box<dyn FnMut(Rect) -> f32>),
    /// Assistive technology, or a test, asks something of the node.
    Semantic(Box<dyn FnMut(&SemanticAction) -> bool>),
    /// An action of a type reached the node.
    Action {
        of: TypeId,
        run: ActionFn,
    },
}

/// What the tree remembers of the pointer between events.
#[derive(Default)]
pub(crate) struct InputState {
    /// Where the pointer is, when it is in the window.
    pub pointer: Option<Point>,
    pub mods: Mods,
    /// The nodes under the pointer, the deepest first.
    pub hover: Vec<NodeId>,
    /// The path of the last press, the deepest first, until its release.
    pub pressed: Vec<NodeId>,
    pub capture: Option<NodeId>,
    /// The last click: when, where, which button, and its count.
    pub last_click: Option<(Instant, Point, MouseButton, u8)>,
}

impl Tree {
    /// Runs `f` for pointer events of a kind that reach the node on their way up from the
    /// target. The handler ends with the node.
    pub fn on_pointer(&mut self, id: NodeId, kind: PointerKind, f: impl FnMut(&mut PointerEvent) + 'static) {
        self.add_handler(id, Handler::Pointer { kind, capture: false, run: Box::new(f) });
    }

    /// As [`Tree::on_pointer`], on the way down from the root: before every other handler.
    pub fn on_pointer_capture(
        &mut self,
        id: NodeId,
        kind: PointerKind,
        f: impl FnMut(&mut PointerEvent) + 'static,
    ) {
        self.add_handler(id, Handler::Pointer { kind, capture: true, run: Box::new(f) });
    }

    pub(crate) fn add_handler(&mut self, id: NodeId, handler: Handler) {
        if let Some(node) = self.get_mut(id) {
            node.handlers.push(handler);
        }
    }

    /// The deepest node at a point of the window: children are tried from the last painted to
    /// the first, then the node itself. It reads layout rects and layer offsets, so it agrees
    /// with what the last frame drew, scrolled content included.
    pub fn hit_test(&self, at: Point) -> Option<NodeId> {
        // What is on top is tried first: the overlays from the last opened, then the main
        // tree. The preview of a drag is not there for the pointer.
        let mut above = self.overlays().rev().filter(|&root| !self.is_drag_preview(root));
        let over = above.find_map(|root| self.hit(root, at));
        over.or_else(|| self.root.and_then(|root| self.hit(root, at)))
    }

    /// `at` is in the space of the node's parent.
    fn hit(&self, id: NodeId, at: Point) -> Option<NodeId> {
        let node = &self.nodes[id.index as usize];
        let style = self.layout.style(node.layout);
        if style.display == Display::None {
            return None;
        }
        let rect = self.layout.rect(node.layout);
        let offset = node.layer.as_ref().map_or(Point::ZERO, |layer| layer.offset);
        let local = Point::new(at.x - rect.x - offset.x, at.y - rect.y - offset.y);
        let inside = local.x >= 0.0 && local.y >= 0.0 && local.x < rect.width && local.y < rect.height;
        // What a node cuts off cannot be hit. ponytail: the box, not its rounded corners.
        let cuts = style.overflow_x != Overflow::Visible || style.overflow_y != Overflow::Visible;
        if cuts && !inside {
            return None;
        }
        let under = node.children.iter().rev().find_map(|&child| self.hit(child, local));
        under.or(inside.then_some(id))
    }

    /// A node and its ancestors, the node first.
    pub(crate) fn path(&self, id: NodeId) -> Vec<NodeId> {
        std::iter::successors(Some(id), |&at| self.nodes[at.index as usize].parent).collect()
    }

    /// Sets the shape the pointer has over a node and what is under it; `None` leaves it to
    /// the node around it.
    pub fn set_cursor(&mut self, id: NodeId, cursor: Option<Cursor>) {
        if let Some(node) = self.get_mut(id) {
            node.cursor = cursor;
        }
    }

    /// The shape the pointer has now: that of the deepest node under it that sets one. It
    /// stays what it was while a node holds the pointer.
    pub fn cursor(&self) -> Cursor {
        let set = self.input.hover.iter().find_map(|&id| self.get(id)?.cursor);
        set.unwrap_or_default()
    }

    /// True while the pointer is over the node or something under it.
    pub fn is_hovered(&self, id: NodeId) -> bool {
        self.input.hover.contains(&id)
    }

    /// True from a press on the node, or on something under it, to its release.
    pub fn is_pressed(&self, id: NodeId) -> bool {
        self.input.pressed.contains(&id)
    }
}
