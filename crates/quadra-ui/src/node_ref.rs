//! A way from app code to a mounted node (specs/04, "Node handles").

use std::cell::RefCell;
use std::rc::Rc;

use quadra_tree::{Handle, NodeId};

/// Filled when the view it was given to is mounted. Every method does nothing while the node
/// is not there, or once it is gone.
#[derive(Clone, Default)]
pub struct NodeRef(Rc<RefCell<Option<(NodeId, Handle)>>>);

impl NodeRef {
    pub fn new() -> NodeRef {
        NodeRef::default()
    }

    pub(crate) fn fill(&self, id: NodeId, handle: Handle) {
        *self.0.borrow_mut() = Some((id, handle));
    }

    /// The node, once it is mounted.
    pub fn get(&self) -> Option<NodeId> {
        self.0.borrow().as_ref().map(|(id, _)| *id)
    }

    /// Scrolls what scrolls around the node so that it shows, in the window's next frame:
    /// after its layout, so a node just mounted is where it will be.
    pub fn scroll_into_view(&self) {
        if let Some((id, handle)) = &*self.0.borrow() {
            let id = *id;
            handle.with(move |tree| tree.after_layout(move |tree| tree.scroll_into_view(id)));
        }
    }

    /// Gives the node the focus, in the window's next frame.
    pub fn focus(&self) {
        if let Some((id, handle)) = &*self.0.borrow() {
            let id = *id;
            handle.with(move |tree| tree.focus(id));
        }
    }
}
