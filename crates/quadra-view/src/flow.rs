//! Control flow (specs/05, "Control flow"): a component runs once, so structure that changes
//! with state is a view of its own.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

use quadra_reactive::{Scope, Signal, memo, signal, untrack};
use quadra_tree::{NodeId, Tree};

use crate::div::{Div, div};
use crate::view::{AnyView, MountCx, View, mount_scoped};

/// Makes the binding that mounts a region's content, once its box is mounted.
type Bind = Box<dyn FnOnce(NodeId, &mut MountCx)>;

/// A region whose content is mounted by a binding, inside a box.
pub struct Region {
    container: Div,
    bind: Bind,
}

impl Region {
    pub(crate) fn new(container: Div, bind: impl FnOnce(NodeId, &mut MountCx) + 'static) -> Region {
        Region { container, bind: Box::new(bind) }
    }

    /// Replaces the box the content is mounted in (a column by default for `each`).
    pub fn container(mut self, container: Div) -> Self {
        self.container = container;
        self
    }
}

impl View for Region {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let id = self.container.mount(cx);
        (self.bind)(id, cx);
        id
    }
}

struct Row<T, K> {
    key: K,
    node: NodeId,
    item: Signal<T>,
}

/// A keyed list. A row is mounted once per key and keeps its node while the key stays; it
/// gets its item as a signal, so a changed item updates the row's bindings in place. A change
/// costs the length of the list: for thousands of rows use a virtual list.
pub fn each<T, K, V>(
    items: impl Fn() -> Vec<T> + 'static,
    key: impl Fn(&T) -> K + 'static,
    row: impl Fn(Signal<T>) -> V + 'static,
) -> Region
where
    T: PartialEq + 'static,
    K: Eq + Hash + Clone + 'static,
    V: View + 'static,
{
    let bind = move |container: NodeId, cx: &mut MountCx| {
        let owner = Scope::current();
        let rows: Rc<RefCell<Vec<Row<T, K>>>> = Rc::default();
        let row = Rc::new(row);
        let writer = cx.handle.clone();
        cx.handle.bind(move || {
            let items = items();
            let keys: Vec<K> = items.iter().map(&key).collect();
            let (rows, row) = (rows.clone(), row.clone());
            writer.with(move |tree| reconcile(tree, container, owner, &rows, items, keys, &*row));
        });
    };
    Region { container: div().direction(quadra_layout::Direction::Column), bind: Box::new(bind) }
}

fn reconcile<T, K, V>(
    tree: &mut Tree,
    container: NodeId,
    owner: Scope,
    rows: &RefCell<Vec<Row<T, K>>>,
    items: Vec<T>,
    keys: Vec<K>,
    row: &dyn Fn(Signal<T>) -> V,
) where
    T: PartialEq + 'static,
    K: Eq + Hash + Clone,
    V: View,
{
    if !tree.is_alive(container) || !owner.is_alive() {
        return;
    }
    let mut old: HashMap<K, Row<T, K>> = rows.borrow_mut().drain(..).map(|r| (r.key.clone(), r)).collect();
    let mut next = Vec::with_capacity(items.len());
    let mut cx = MountCx::new(tree);
    for (item, key) in items.into_iter().zip(keys) {
        next.push(match old.remove(&key) {
            Some(kept) => {
                kept.item.set(item); // nothing happens when it is equal
                kept
            }
            None => {
                // The row's signal and its bindings live in a scope that ends with its node.
                let scope = owner.run(Scope::child);
                let (item, node) = scope.run(|| {
                    let item = signal(item);
                    (item, row(item).mount(&mut cx))
                });
                cx.tree.set_scope(node, scope);
                Row { key, node, item }
            }
        });
    }
    for gone in old.into_values() {
        cx.tree.remove(gone.node);
    }
    let nodes: Vec<NodeId> = next.iter().map(|r| r.node).collect();
    cx.tree.set_children(container, &nodes);
    *rows.borrow_mut() = next;
}

fn region(content: impl Fn() -> Option<AnyView> + 'static) -> Region {
    let bind = move |container: NodeId, cx: &mut MountCx| {
        let owner = Scope::current();
        let mounted: Rc<Cell<Option<NodeId>>> = Rc::default();
        let writer = cx.handle.clone();
        cx.handle.bind(move || {
            // The view is built inside the scope that will end with it: signals a component
            // creates while it is built belong to the region, not to whoever runs the binding.
            let scope = owner.run(Scope::child);
            let view = scope.run(&content);
            let mounted = mounted.clone();
            writer.with(move |tree| {
                if let Some(old) = mounted.take() {
                    tree.remove(old);
                }
                // The box may have gone in this very frame: a row of a list that is built
                // again while the list drops it.
                let there = tree.is_alive(container);
                let Some(view) = view.filter(|_| there) else {
                    scope.dispose();
                    if there {
                        tree.set_children(container, &[]);
                    }
                    return;
                };
                let node = scope.run(|| view.mount(&mut MountCx::new(tree)));
                tree.set_scope(node, scope);
                tree.set_children(container, &[node]);
                mounted.set(Some(node));
            });
        });
    };
    Region { container: div(), bind: Box::new(bind) }
}

/// The coarse tool: whatever `view` reads, a change removes the region's content and mounts
/// it again. Node state inside is lost each time; state that must survive lives in signals
/// created outside.
pub fn dynamic<V: View + 'static>(view: impl Fn() -> V + 'static) -> Region {
    region(move || Some(AnyView::new(view())))
}

/// A part that appears and disappears. It is mounted when `condition` becomes true and
/// removed when it becomes false; what the part itself reads does not rebuild it.
pub fn show<V: View + 'static>(
    condition: impl Fn() -> bool + 'static,
    view: impl Fn() -> V + 'static,
) -> Region {
    let on = memo(condition);
    region(move || on.get().then(|| AnyView::new(untrack(&view))))
}

/// Mounts `view` under `owner` in a scope of its own: for code that mounts by hand.
pub fn mount_in(cx: &mut MountCx, owner: Scope, view: impl View) -> NodeId {
    mount_scoped(cx, owner, view)
}
