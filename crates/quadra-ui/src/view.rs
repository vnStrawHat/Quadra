//! Views: descriptions consumed once to mount nodes (specs/05, "Views").

use quadra_reactive::Scope;
use quadra_tree::{Handle, NodeId, Tree};

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

/// Mounts a view as the root of a tree.
pub fn mount_root(tree: &mut Tree, view: impl View) -> NodeId {
    let mut cx = MountCx::new(tree);
    let id = mount_scoped(&mut cx, Scope::current(), view);
    tree.set_root(id);
    id
}
