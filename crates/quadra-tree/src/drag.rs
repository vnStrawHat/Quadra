//! Drag and drop inside a window (specs/09, "Drag and drop"): a press on something that can
//! be dragged becomes a drag once the pointer has moved far enough. What is dragged is a
//! value; the nodes that take values of its type are its targets: the one under the pointer
//! has the state `DROP_TARGET`, and gets the value at the release. A preview follows the
//! pointer as an overlay that is neither hit nor read.

use std::any::{Any, TypeId};
use std::rc::Rc;

use quadra_prim::Point;

use crate::input::Handler;
use crate::node::NodeId;
use crate::overlay::Anchor;
use crate::style::State;
use crate::tree::Tree;

/// How far the pointer moves with the button down before a press is a drag, in pixels.
const THRESHOLD: f32 = 4.0;

/// A press on something that can be dragged, and the drag it became.
pub(crate) struct Drag {
    source: NodeId,
    pressed_at: Point,
    /// From the corner of what is dragged to where it was pressed: the preview is held
    /// there too.
    held: Point,
    /// What is dragged, once the pointer has moved past the threshold.
    payload: Option<Rc<dyn Any>>,
    preview: Option<NodeId>,
    /// The node under the pointer that takes the payload.
    over: Option<NodeId>,
}

impl Tree {
    /// Lets a node be dragged: `payload` is asked for the value when a drag of it starts.
    pub fn on_drag(&mut self, id: NodeId, payload: impl FnMut() -> Rc<dyn Any> + 'static) {
        self.add_handler(id, Handler::Drag(Box::new(payload)));
    }

    /// Lets a node take what is dragged when it is a `T`: `f` runs with the value and the
    /// point of the window where it was let go.
    pub fn on_drop<T: 'static>(&mut self, id: NodeId, mut f: impl FnMut(&T, Point) + 'static) {
        let run = move |payload: &dyn Any, at: Point| {
            if let Some(value) = payload.downcast_ref::<T>() {
                f(value, at);
            }
        };
        self.add_handler(id, Handler::Drop { of: TypeId::of::<T>(), run: Box::new(run) });
    }

    /// Whether something is being dragged.
    pub fn dragging(&self) -> bool {
        self.drag.as_ref().is_some_and(|drag| drag.payload.is_some())
    }

    /// Shows a node beside the pointer while the drag that is under way lasts: it is removed
    /// when the drag ends, and at once when there is none.
    pub fn set_drag_preview(&mut self, root: NodeId) {
        let at = self.input.pointer.unwrap_or_default();
        match self.drag.as_mut().filter(|drag| drag.payload.is_some()) {
            Some(drag) => {
                let (old, at) = (drag.preview.replace(root), at - drag.held);
                self.open_overlay(root, Anchor::Free(at));
                old.into_iter().for_each(|old| self.remove(old));
            }
            None => self.remove(root),
        }
    }

    /// Whether a node is the root of the preview of a drag: nothing of it is hit or read.
    pub(crate) fn is_drag_preview(&self, root: NodeId) -> bool {
        self.drag.as_ref().is_some_and(|drag| drag.preview == Some(root))
    }

    /// A press: it may become a drag of the nearest node on its path that can be dragged.
    pub(crate) fn drag_press(&mut self, path: &[NodeId], at: Point) {
        let can = |id: &&NodeId| {
            self.nodes[id.index as usize].handlers.iter().any(|h| matches!(h, Handler::Drag(_)))
        };
        self.drag = path.iter().find(can).map(|&source| Drag {
            source,
            pressed_at: at,
            held: at - self.window_rect(source).origin(),
            payload: None,
            preview: None,
            over: None,
        });
    }

    /// The pointer moved with the button down. Returns whether a drag is under way: the
    /// move is then the drag's, and no node's.
    pub(crate) fn drag_move(&mut self, at: Point) -> bool {
        let Some(drag) = &self.drag else { return false };
        if drag.payload.is_none() {
            let (source, from) = (drag.source, drag.pressed_at);
            if (at.x - from.x).hypot(at.y - from.y) <= THRESHOLD || !self.is_alive(source) {
                return false;
            }
            // The press is a drag now, and no click: what it held is let go.
            let mut payload = None;
            for handler in &mut self.nodes[source.index as usize].handlers {
                if let Handler::Drag(start) = handler {
                    payload = Some(start());
                }
            }
            for held in std::mem::take(&mut self.input.pressed) {
                self.set_state(held, State::ACTIVE, false);
            }
            self.input.capture = None;
            if let Some(drag) = &mut self.drag {
                drag.payload = payload;
            }
        }
        let Some(drag) = &self.drag else { return false };
        let (Some(payload), preview, held) = (drag.payload.clone(), drag.preview, drag.held) else {
            return false;
        };
        if let Some(preview) = preview {
            self.move_overlay(preview, Anchor::Free(at - held));
        }
        // The deepest node under the pointer that takes a value of this type.
        let of = payload.as_ref().type_id();
        let takes = |id: &NodeId| {
            let handlers = &self.nodes[id.index as usize].handlers;
            handlers.iter().any(|h| matches!(h, Handler::Drop { of: takes, .. } if *takes == of))
        };
        let over = self.hit_test(at).and_then(|under| self.path(under).into_iter().find(takes));
        let was = self.drag.as_mut().and_then(|drag| std::mem::replace(&mut drag.over, over));
        if was != over {
            for (node, on) in [(was, false), (over, true)] {
                if let Some(node) = node.filter(|&node| self.is_alive(node)) {
                    self.set_state(node, State::DROP_TARGET, on);
                }
            }
        }
        true
    }

    /// The button was released. Returns whether that ended a drag: the target under the
    /// pointer, if there is one, was given the value, and there is no click.
    pub(crate) fn drag_release(&mut self, at: Point) -> bool {
        if !self.dragging() {
            self.drag = None; // a press that never became a drag
            return false;
        }
        self.drag_move(at);
        let Some(drag) = self.drag_end() else { return true };
        if let (Some(target), Some(payload)) = (drag.over.filter(|&n| self.is_alive(n)), drag.payload) {
            for handler in &mut self.nodes[target.index as usize].handlers {
                if let Handler::Drop { of, run } = handler
                    && *of == payload.as_ref().type_id()
                {
                    run(payload.as_ref(), at);
                }
            }
        }
        true
    }

    /// Gives a drag up, as Escape does: nothing is dropped. Returns whether there was one.
    pub(crate) fn drag_cancel(&mut self) -> bool {
        self.dragging() && self.drag_end().is_some()
    }

    /// Ends the drag: its target is one no longer, and its preview is gone.
    fn drag_end(&mut self) -> Option<Drag> {
        let drag = self.drag.take()?;
        if let Some(over) = drag.over.filter(|&node| self.is_alive(node)) {
            self.set_state(over, State::DROP_TARGET, false);
        }
        if let Some(preview) = drag.preview {
            self.remove(preview);
        }
        Some(drag)
    }
}
