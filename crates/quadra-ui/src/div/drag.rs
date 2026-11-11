//! Dragging a box, and dropping on one (specs/09, "Drag and drop").

use std::any::Any;
use std::rc::Rc;

use quadra_prim::Point;
use quadra_reactive::Scope;
use quadra_tree::State;

use super::{Div, Refine};
use crate::view::{MountCx, View};

impl Div {
    /// Lets the box be dragged. `payload` gives the value a drag of it carries; the boxes
    /// that take values of that type are where it can be dropped.
    pub fn draggable<T: 'static>(self, payload: impl Fn() -> T + 'static) -> Self {
        self.on_mount(move |tree, id| tree.on_drag(id, move || Rc::new(payload()) as Rc<dyn Any>))
    }

    /// As [`Div::draggable`], with a view that follows the pointer while the drag lasts. It
    /// is built when the drag starts; it is not there for the pointer, nor for a reader.
    pub fn draggable_with<T: 'static, V: View + 'static>(
        self,
        payload: impl Fn() -> T + 'static,
        preview: impl Fn() -> V + 'static,
    ) -> Self {
        self.on_mount(move |tree, id| {
            let (owner, handle, preview) = (Scope::current(), tree.handle(), Rc::new(preview));
            tree.on_drag(id, move || {
                let preview = preview.clone();
                handle.with(move |tree| {
                    if owner.is_alive() {
                        // What the preview binds lives in a scope that ends with its nodes.
                        let scope = owner.run(Scope::child);
                        let root = scope.run(|| preview().mount(&mut MountCx::new(tree)));
                        tree.set_scope(root, scope);
                        tree.set_drag_preview(root);
                    }
                });
                Rc::new(payload()) as Rc<dyn Any>
            });
        })
    }

    /// Lets the box take what is dragged when it is a `T`: `f` runs with the value, and with
    /// where in the window it was let go.
    pub fn on_drop<T: 'static>(self, f: impl FnMut(&T, Point) + 'static) -> Self {
        self.on_mount(move |tree, id| tree.on_drop::<T>(id, f))
    }

    /// What the box looks like while something it takes is dragged over it.
    pub fn drop_target(self, refine: impl FnOnce(Refine) -> Refine) -> Self {
        self.when(State::DROP_TARGET, refine)
    }
}
