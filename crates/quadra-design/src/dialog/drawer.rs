//! The drawer (specs/behaviour/15, "Dialog"): a dialog along an edge of the window, over
//! it or beside the content.

use std::rc::Rc;

use quadra_headless::dialog as base;
use quadra_prim::{Ease, Point};
use quadra_reactive::{Signal, on_cleanup};
use quadra_tree::NodeId;
use quadra_view::{AnyView, Div, MountCx, NodeRef, View, column, div, row, show, timeline};

use super::head;
use crate::popup::{EDGE, Edge, popup};
use crate::token;

type Head = Rc<dyn Fn() -> AnyView>;

pub struct Drawer<F> {
    open: Signal<bool>,
    edge: Edge,
    title: String,
    size: f32,
    body: F,
    beside: bool,
    own: Option<Head>,
}

/// A dialog that lies along an edge of the window, over all of its length, while `open` is
/// true: a title over what `body` builds. It reaches 360 px into the window.
pub fn drawer<V, F>(open: Signal<bool>, edge: Edge, title: impl Into<String>, body: F) -> Drawer<F>
where
    V: View + 'static,
    F: Fn() -> V + 'static,
{
    Drawer { open, edge, title: title.into(), size: 360.0, body, beside: false, own: None }
}

impl<F> Drawer<F> {
    /// How far the drawer reaches into the window, with its border.
    pub fn size(mut self, px: f32) -> Self {
        self.size = px;
        self
    }

    /// In the layout where it stands, next to the content, not over the window: nothing
    /// outside closes it and the focus is not kept in it.
    pub fn beside(mut self) -> Self {
        self.beside = true;
        self
    }

    /// A row of the app's in the place of the title, before the cross.
    pub fn head<H: View + 'static>(mut self, row: impl Fn() -> H + 'static) -> Self {
        self.own = Some(Rc::new(move || AnyView::new(row())));
        self
    }
}

/// A drawer beside the content while it is open: the focus goes into it, and back to what
/// had it when it closes, unless it went elsewhere.
struct Beside {
    page: Div,
}

impl View for Beside {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (before, mine) = (cx.tree.focused(), NodeRef::new());
        let id = self.page.node_ref(&mine).mount(cx);
        cx.tree.focus_first(id);
        let handle = cx.handle.clone();
        on_cleanup(move || {
            handle.with(move |tree| {
                // Gone with the drawer, or still in it: not where the user put it since.
                let inside = |tree: &quadra_tree::Tree, mut at: NodeId| loop {
                    match Some(at) == mine.get() {
                        true => break true,
                        false => match tree.parent(at) {
                            Some(up) => at = up,
                            None => break false,
                        },
                    }
                };
                let lost = tree.focused().is_none_or(|at| !tree.is_alive(at) || inside(tree, at));
                if let Some(before) = before.filter(|&it| lost && tree.is_alive(it)) {
                    tree.focus(before);
                }
            })
        });
        id
    }
}

impl<V, F> View for Drawer<F>
where
    V: View + 'static,
    F: Fn() -> V + 'static,
{
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let Drawer { open, edge, title, size, body, beside, own } = self;
        let content = move || {
            let page = base::page(column().grow(1.0), title.clone());
            // Beside the content the page is the drawer, its line within its size.
            let line = if beside { 0.0 } else { EDGE };
            let page = match edge {
                Edge::Bottom => page.height(size - line),
                _ => page.width(size - line),
            };
            let middle = div().grow(1.0).padding_x(12.0).padding_y(8.0).text_color(token::TEXT_2);
            // It slides in from its edge: put past it, then moved to its place over 200 ms.
            let away = match edge {
                Edge::Left => Point::new(-size, 0.0),
                Edge::Right => Point::new(size, 0.0),
                Edge::Bottom => Point::new(0.0, size),
            };
            let slide = timeline().to(0, Ease::Linear, |s| s.translate(away));
            let page = page.animate(slide.to(200, Ease::Out, |s| s.translate(Point::ZERO)));
            page.child(head(&title, own.as_ref().map(|own| own()), open)).child(middle.child(body()))
        };
        if !beside {
            return popup(open, edge, None, content).mount(cx);
        }
        let shown = move || {
            let page = content().shrink(0.0).bg(token::PANEL).border_color(token::LINE_STRONG);
            let page = match edge {
                Edge::Left => page.border_right(1.0),
                Edge::Right => page.border_left(1.0),
                Edge::Bottom => page.border_top(1.0),
            };
            // Escape gives it up while the focus is in it.
            let page = page.tab_index(-1).key_context(quadra_headless::action::POPUP);
            Beside { page: page.on_action::<quadra_headless::action::Cancel>(move |_| open.set(false)) }
        };
        // Closed, it takes no room.
        let stand = match edge {
            Edge::Bottom => column().shrink(0.0),
            _ => row().shrink(0.0),
        };
        show(move || open.get(), shown).container(stand).mount(cx)
    }
}
