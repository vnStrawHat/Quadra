//! The box view: a node that is its style and its children (specs/05).

use quadra_layout::{Align, Direction, Justify, Length, Overflow};
use quadra_platform::MouseButton;
use quadra_prim::{Color, Corners, Edges};
use quadra_tree::{Handle, NodeId, PaintStyle, PointerEvent, PointerKind, Style};

use crate::prop::{IntoProp, Prop};
use crate::view::{AnyView, MountCx, View};

/// Makes a binding for one node once it is mounted.
type Binder = Box<dyn FnOnce(NodeId, &Handle)>;
type PointerFn = Box<dyn FnMut(&mut PointerEvent)>;

#[derive(Default)]
pub struct Div {
    style: Style,
    children: Vec<AnyView>,
    binders: Vec<Binder>,
    handlers: Vec<(PointerKind, PointerFn)>,
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

/// Declares setters for layout values: each takes a constant or something reactive.
macro_rules! layout_props {
    ($($(#[$doc:meta])* $name:ident: $ty:ty => |$s:ident, $v:ident| $set:expr;)*) => {$(
        $(#[$doc])*
        pub fn $name<M>(mut self, value: impl IntoProp<$ty, M>) -> Self {
            self.layout_prop(value.into_prop(), |$s, $v| $set);
            self
        }
    )*};
}

macro_rules! paint_props {
    ($($(#[$doc:meta])* $name:ident: $ty:ty => |$s:ident, $v:ident| $set:expr;)*) => {$(
        $(#[$doc])*
        pub fn $name<M>(mut self, value: impl IntoProp<$ty, M>) -> Self {
            self.paint_prop(value.into_prop(), |$s, $v| $set);
            self
        }
    )*};
}

impl Div {
    fn layout_prop<T: 'static>(&mut self, prop: Prop<T>, set: fn(&mut quadra_layout::Style, T)) {
        match prop {
            Prop::Value(value) => set(&mut self.style.layout, value),
            Prop::Reactive(read) => self.binders.push(Box::new(move |id, handle| {
                let writer = handle.clone();
                handle.bind(move || {
                    let value = read();
                    // ponytail: one boxed write per run; a typed write per property if the
                    // grid bench shows it.
                    writer.with(move |tree| {
                        if tree.is_alive(id) {
                            let mut style = tree.layout_style(id).clone();
                            set(&mut style, value);
                            tree.set_layout_style(id, style);
                        }
                    });
                });
            })),
        }
    }

    fn paint_prop<T: 'static>(&mut self, prop: Prop<T>, set: fn(&mut PaintStyle, T)) {
        match prop {
            Prop::Value(value) => set(&mut self.style.paint, value),
            Prop::Reactive(read) => self.binders.push(Box::new(move |id, handle| {
                let writer = handle.clone();
                handle.bind(move || {
                    let value = read();
                    writer.with(move |tree| {
                        if tree.is_alive(id) {
                            let mut style = tree.paint_style(id);
                            set(&mut style, value);
                            tree.set_paint_style(id, style);
                        }
                    });
                });
            })),
        }
    }

    layout_props! {
        width: Length => |s, v| s.width = v;
        height: Length => |s, v| s.height = v;
        min_width: Length => |s, v| s.min_width = v;
        min_height: Length => |s, v| s.min_height = v;
        max_width: Length => |s, v| s.max_width = v;
        max_height: Length => |s, v| s.max_height = v;
        /// How much of the free space the box takes.
        grow: f32 => |s, v| s.grow = v;
        shrink: f32 => |s, v| s.shrink = v;
        /// The space between children, on both axes.
        gap: f32 => |s, v| (s.gap_row, s.gap_col) = (v, v);
        /// The same padding on every side.
        padding: f32 => |s, v| s.padding = Edges::all(v);
        /// Left and right padding.
        padding_x: f32 => |s, v| (s.padding.left, s.padding.right) = (v, v);
        /// Top and bottom padding.
        padding_y: f32 => |s, v| (s.padding.top, s.padding.bottom) = (v, v);
        margin: f32 => |s, v| s.margin = Edges::all(v);
        /// The width of the border on every side.
        border: f32 => |s, v| s.border = Edges::all(v);
        direction: Direction => |s, v| s.direction = v;
        justify: Justify => |s, v| s.justify = v;
        align_items: Align => |s, v| s.align_items = v;
        wrap: bool => |s, v| s.wrap = v;
        overflow_y: Overflow => |s, v| s.overflow_y = v;
    }

    paint_props! {
        /// The background colour.
        bg: Color => |s, v| s.background = v;
        border_color: Color => |s, v| s.border_color = v;
        /// The same radius on every corner.
        radius: f32 => |s, v| s.radii = Corners::all(v);
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
    pub fn on_pointer(mut self, kind: PointerKind, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
        self.handlers.push((kind, Box::new(f)));
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
        for (kind, run) in self.handlers {
            cx.tree.on_pointer(id, kind, run);
        }
        id
    }
}
