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
        self.draggable_off(0.0, payload, preview)
    }

    /// As [`Div::draggable_with`], for a box that is torn off what it is in: the drag starts
    /// once the pointer is `off` pixels above or below where it pressed, and the moves are
    /// the box's own until then. Zero is the usual threshold, any way.
    pub fn draggable_off<T: 'static, V: View + 'static>(
        self,
        off: f32,
        payload: impl Fn() -> T + 'static,
        preview: impl Fn() -> V + 'static,
    ) -> Self {
        self.on_mount(move |tree, id| {
            let (owner, handle, preview) = (Scope::current(), tree.handle(), Rc::new(preview));
            tree.on_drag_off(id, off, move || {
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

    /// Tells the box where the pointer is in the window while a `T` is dragged over it, and
    /// `None` when the pointer leaves it or the drag ends: to show where a drop would land.
    pub fn on_drag_over<T: 'static>(self, f: impl FnMut(Option<Point>) + 'static) -> Self {
        self.on_mount(move |tree, id| tree.on_drag_over::<T>(id, f))
    }

    /// What the box looks like while something it takes is dragged over it.
    pub fn drop_target(self, refine: impl FnOnce(Refine) -> Refine) -> Self {
        self.when(State::DROP_TARGET, refine)
    }
}
