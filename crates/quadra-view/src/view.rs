//! Views: descriptions consumed once to mount nodes (specs/05, "Views").

use std::cell::Cell;
use std::rc::Rc;

use quadra_reactive::{Scope, expect, on_cleanup, provide};
use quadra_tree::{Handle, NodeId, Tree};

use crate::window::Window;

/// What a view mounts into.
pub struct MountCx<'a> {
    pub tree: &'a mut Tree,
    pub handle: Handle,
}

impl MountCx<'_> {
    pub fn new(tree: &mut Tree) -> MountCx<'_> {
        let handle = tree.handle();
        MountCx { tree, handle }
    }
}

/// A description of UI. It is consumed when mounted: it creates nodes and binds their
/// properties to the signals they read. After that the signals drive the nodes.
pub trait View {
    fn mount(self, cx: &mut MountCx) -> NodeId;
}

/// A view of any type. Containers box their children, so a tree of views is not one large
/// generic type and adding a child instantiates no new code (specs/01, R17).
pub struct AnyView(Box<dyn FnOnce(&mut MountCx) -> NodeId>);

impl AnyView {
    pub fn new(view: impl View + 'static) -> AnyView {
        AnyView(Box::new(move |cx| view.mount(cx)))
    }
}

impl View for AnyView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        (self.0)(cx)
    }
}

/// Mounts a view in a scope of its own, which ends with the node: what the view created
/// (signals, bindings) lives exactly as long as what it mounted.
pub fn mount_scoped(cx: &mut MountCx, owner: Scope, view: impl View) -> NodeId {
    let scope = owner.run(Scope::child);
    let id = scope.run(|| view.mount(cx));
    cx.tree.set_scope(id, scope);
    id
}

/// Builds a view where it can reach the tree, and mounts it as the tree's root: `build` runs
/// in the root's scope, which provides the tree's [`Handle`] as context. This is how a
/// window's view is mounted; [`mount_root`] is for a view built beforehand.
pub fn mount_with<V: View>(tree: &mut Tree, build: impl FnOnce() -> V) -> NodeId {
    let mut cx = MountCx::new(tree);
    let scope = Scope::child();
    let id = scope.run(|| {
        provide(cx.handle.clone());
        provide(Window::new(cx.handle.clone(), cx.tree.window_state()));
        build().mount(&mut cx)
    });
    cx.tree.set_scope(id, scope);
    tree.set_root(id);
    id
}

/// Runs `f` at the start of each frame of the window, in the frame's batch, until it returns
/// false or the scope it was called in ends (specs/05, "Frame callbacks"). While one runs,
/// the window draws every frame the display can take.
///
/// Call it where the window's tree is in context: in a component of a view built by
/// [`mount_with`], which is every view of an app's window.
#[track_caller]
pub fn each_frame(mut f: impl FnMut() -> bool + 'static) {
    let handle = expect::<Handle>();
    let (scope, alive) = (Scope::current(), Rc::new(Cell::new(true)));
    let ended = alive.clone();
    on_cleanup(move || ended.set(false));
    // It runs in the scope it was registered in: what it creates belongs there.
    handle.each_frame(move || alive.get() && scope.run(&mut f));
}

/// Mounts a view as the root of a tree.
pub fn mount_root(tree: &mut Tree, view: impl View) -> NodeId {
    let mut cx = MountCx::new(tree);
    let id = mount_scoped(&mut cx, Scope::current(), view);
    tree.set_root(id);
    id
}
