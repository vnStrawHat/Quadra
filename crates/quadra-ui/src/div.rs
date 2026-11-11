//! The box view: a node that is its style and its children (specs/05).

use quadra_layout::{Align, Direction, Justify, Length, Overflow};
use quadra_platform::{Cursor, MouseButton};
use quadra_prim::Color;
use quadra_tree::{Action, Handle, KeyEvent, NodeId, Patch, PointerEvent, PointerKind, State, Style, Tree};

use crate::node_ref::NodeRef;

mod drag;
mod says;

use crate::prop::{IntoProp, Prop};
use crate::view::{AnyView, MountCx, View};

/// Makes a binding for one node once it is mounted.
type Binder = Box<dyn FnOnce(NodeId, &Handle)>;
/// Something to do with the node once it is mounted: a handler to attach, a flag to set.
type Mount = Box<dyn FnOnce(&mut Tree, NodeId)>;

#[derive(Default)]
pub struct Div {
    style: Style,
    children: Vec<AnyView>,
    pub(crate) binders: Vec<Binder>,
    mounts: Vec<Mount>,
    variants: Vec<(State, Vec<Patch>)>,
}

/// A box. Its children are laid out in a row unless a direction says otherwise.
pub fn div() -> Div {
    Div::default()
}

pub fn row() -> Div {
    div()
}

pub fn column() -> Div {
    div().direction(Direction::Column)
}

/// What a state changes of a box's style: the setters of the box, taking constants (specs/05,
/// "Style and states").
#[derive(Default)]
pub struct Refine(Vec<Patch>);

/// Declares the style values: each is a setter of the box, taking a constant or something
/// reactive, and a setter of a refinement, taking a constant.
macro_rules! props {
    ($($(#[$doc:meta])* $name:ident: $ty:ty => $patch:ident;)*) => {
        impl Div {$(
            $(#[$doc])*
            pub fn $name<M>(mut self, value: impl IntoProp<$ty, M>) -> Self {
                self.prop(value.into_prop(), Patch::$patch);
                self
            }
        )*}

        impl Refine {$(
            $(#[$doc])*
            pub fn $name(mut self, value: impl Into<$ty>) -> Self {
                self.0.push(Patch::$patch(value.into()));
                self
            }
        )*}
    };
}

props! {
    width: Length => Width;
    height: Length => Height;
    min_width: Length => MinWidth;
    min_height: Length => MinHeight;
    max_width: Length => MaxWidth;
    max_height: Length => MaxHeight;
    /// How much of the free space the box takes.
    grow: f32 => Grow;
    shrink: f32 => Shrink;
    /// The space between children, on both axes.
    gap: f32 => Gap;
    /// The same padding on every side.
    padding: f32 => Padding;
    /// Left and right padding.
    padding_x: f32 => PaddingX;
    /// Top and bottom padding.
    padding_y: f32 => PaddingY;
    margin: f32 => Margin;
    /// The width of the border on every side.
    border: f32 => Border;
    direction: Direction => Direction;
    justify: Justify => Justify;
    align_items: Align => AlignItems;
    wrap: bool => Wrap;
    overflow_y: Overflow => OverflowY;
    /// The background colour.
    bg: Color => Background;
    border_color: Color => BorderColor;
    /// The same radius on every corner.
    radius: f32 => Radius;
}

impl Div {
    /// A constant goes into the style the node is mounted with; something reactive becomes a
    /// binding that writes that one value when it changes.
    fn prop<T: 'static>(&mut self, prop: Prop<T>, patch: fn(T) -> Patch) {
        match prop {
            Prop::Value(value) => drop(patch(value).apply(&mut self.style)),
            Prop::Reactive(read) => self.binders.push(Box::new(move |id, handle| {
                let writer = handle.clone();
                handle.bind(move || writer.patch_style(id, patch(read())));
            })),
        }
    }

    /// What the box looks like while a state is set: the values `refine` sets are applied
    /// over the box's own. `refine` runs once, here; no app code runs when the state changes.
    pub fn when(mut self, state: State, refine: impl FnOnce(Refine) -> Refine) -> Self {
        self.variants.push((state, refine(Refine::default()).0));
        self
    }

    /// While the pointer is over the box.
    pub fn hover(self, refine: impl FnOnce(Refine) -> Refine) -> Self {
        self.when(State::HOVER, refine)
    }

    /// While the box has the focus.
    pub fn focus(self, refine: impl FnOnce(Refine) -> Refine) -> Self {
        self.when(State::FOCUS, refine)
    }

    /// While the box has a focus that came by the keyboard: where a focus ring is drawn.
    pub fn focus_visible(self, refine: impl FnOnce(Refine) -> Refine) -> Self {
        self.when(State::FOCUS_VISIBLE, refine)
    }

    /// While the box or something in it has the focus.
    pub fn focus_within(self, refine: impl FnOnce(Refine) -> Refine) -> Self {
        self.when(State::FOCUS_WITHIN, refine)
    }

    /// While the box is pressed.
    pub fn active(self, refine: impl FnOnce(Refine) -> Refine) -> Self {
        self.when(State::ACTIVE, refine)
    }

    /// A fixed width and height in pixels.
    pub fn size(self, width: f32, height: f32) -> Self {
        self.width(width).height(height)
    }

    /// Runs `f` when the box, or something in it, is clicked with the main button. The
    /// handler writes signals; it ends with the node (specs/05, "Handlers").
    pub fn on_click(self, mut f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
        self.on_pointer(PointerKind::Click, move |e| {
            if e.button == MouseButton::Left {
                f(e)
            }
        })
    }

    /// Runs `f` for pointer events of a kind that reach the box: a press, a release, a move,
    /// the pointer entering or leaving, a click with any button.
    pub fn on_pointer(self, kind: PointerKind, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
        self.on_mount(move |tree, id| tree.on_pointer(id, kind, f))
    }

    /// Runs `f` for key presses that reach the box: it has the focus, or something in it has.
    pub fn on_key_down(self, f: impl FnMut(&mut KeyEvent) + 'static) -> Self {
        self.on_mount(move |tree, id| tree.on_key(id, true, f))
    }

    pub fn on_key_up(self, f: impl FnMut(&mut KeyEvent) + 'static) -> Self {
        self.on_mount(move |tree, id| tree.on_key(id, false, f))
    }

    /// Runs `f` with true when the focus comes to the box, and with false when it leaves.
    pub fn on_focus(self, f: impl FnMut(bool) + 'static) -> Self {
        self.on_mount(move |tree, id| tree.on_focus(id, f))
    }

    /// Runs `f` when an action of type `A` reaches the box: the focus is on it or in it, and
    /// nothing nearer the focus handled the action (specs/09).
    pub fn on_action<A: Action>(self, f: impl FnMut(&A) + 'static) -> Self {
        self.on_mount(move |tree, id| tree.on_action::<A>(id, f))
    }

    /// Names the box as a context of key bindings: those made for it apply while the focus is
    /// in the box.
    pub fn key_context(self, context: &'static str) -> Self {
        self.on_mount(move |tree, id| tree.set_key_context(id, context))
    }

    /// Lets the box have the focus: a click on it focuses it, and Tab stops at it.
    pub fn focusable(self) -> Self {
        self.on_mount(|tree, id| tree.set_focusable(id, true))
    }

    /// Keeps Tab and Shift+Tab among what is in the box: a dialog, a menu.
    /// The shape the pointer has over the box and what it holds.
    pub fn cursor(self, cursor: Cursor) -> Self {
        self.on_mount(move |tree, id| tree.set_cursor(id, Some(cursor)))
    }

    /// Lets the box have the focus, with a place in the order Tab walks: above zero before
    /// the boxes without one, lowest first; below zero never by Tab.
    pub fn tab_index(self, index: i16) -> Self {
        self.on_mount(move |tree, id| tree.set_tab_index(id, index))
    }

    pub fn focus_scope(self) -> Self {
        self.on_mount(|tree, id| tree.set_focus_scope(id, true))
    }

    /// Fills `node` with the box's node when it is mounted: how app code reaches it later.
    pub fn node_ref(self, node: &NodeRef) -> Self {
        let node = node.clone();
        self.on_mount(move |tree, id| node.fill(id, tree.handle()))
    }

    pub(crate) fn on_mount(mut self, f: impl FnOnce(&mut Tree, NodeId) + 'static) -> Self {
        self.mounts.push(Box::new(f));
        self
    }

    pub fn child(mut self, view: impl View + 'static) -> Self {
        self.children.push(AnyView::new(view));
        self
    }

    pub fn children<V: View + 'static>(mut self, views: impl IntoIterator<Item = V>) -> Self {
        self.children.extend(views.into_iter().map(AnyView::new));
        self
    }
}

impl View for Div {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let id = cx.tree.insert(self.style);
        let children: Vec<NodeId> = self.children.into_iter().map(|child| child.mount(cx)).collect();
        cx.tree.set_children(id, &children);
        for bind in self.binders {
            bind(id, &cx.handle);
        }
        for mount in self.mounts {
            mount(cx.tree, id);
        }
        for (state, patches) in self.variants {
            cx.tree.set_variant(id, state, patches);
        }
        id
    }
}
