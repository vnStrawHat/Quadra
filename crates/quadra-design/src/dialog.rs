//! The dialog (specs/behaviour/15, "Dialog"): a question the user answers before going on,
//! and the drawer, which is one along an edge of the window.
//! It hangs 80 px from the top of the window, centred, with no backdrop: the cross at the end
//! of its title, Escape or a press outside gives it up.

use std::rc::Rc;

use quadra_headless::dialog as base;
use quadra_layout::{Align, Justify};
use quadra_platform::Cursor;
use quadra_prim::{Ease, Point};
use quadra_reactive::{Signal, on_cleanup};
use quadra_tree::NodeId;
use quadra_view::{
    AnyView, Div, IntoProp, MountCx, NodeRef, Place, Prop, View, column, div, row, show, text, timeline,
};

use crate::button::{Variant, button};
use crate::icon::icon;
use crate::popup::{EDGE, Edge, popup};
use crate::{icons, size, token};

type Act = Rc<dyn Fn()>;

/// A button at the foot: its label and look, what it does, its key, and when it is off.
struct Action {
    label: String,
    variant: Variant,
    act: Act,
    key: Option<&'static str>,
    off: Option<Rc<dyn Fn() -> bool>>,
}

pub struct Dialog<F> {
    open: Signal<bool>,
    title: String,
    body: F,
    actions: Vec<Action>,
}

/// A dialog shown while `open` is true: a title over what `body` builds, each time it opens.
pub fn dialog<V, F>(open: Signal<bool>, title: impl Into<String>, body: F) -> Dialog<F>
where
    V: View + 'static,
    F: Fn() -> V + 'static,
{
    Dialog { open, title: title.into(), body, actions: Vec::new() }
}

impl<F> Dialog<F> {
    /// A button at the foot of the dialog, from the left to the right in the order given.
    /// Pressing it runs `f` and closes the dialog.
    pub fn action(mut self, label: impl Into<String>, variant: Variant, f: impl Fn() + 'static) -> Self {
        let (label, act) = (label.into(), Rc::new(f));
        self.actions.push(Action { label, variant, act, key: None, off: None });
        self
    }

    /// The key that does what the last action does, shown after its label (`Enter`).
    pub fn shortcut(mut self, key: &'static str) -> Self {
        if let Some(last) = self.actions.last_mut() {
            last.key = Some(key);
        }
        self
    }

    /// The last action is off while `off` says so: it can be neither pressed nor reached.
    pub fn disabled<M>(mut self, off: impl IntoProp<bool, M>) -> Self {
        if let Some(last) = self.actions.last_mut() {
            last.off = Some(match off.into_prop() {
                Prop::Value(off) => Rc::new(move || off),
                Prop::Reactive(read) => Rc::from(read),
            });
        }
        self
    }

    /// The last action destroys something: in the colour of errors.
    pub fn danger(mut self) -> Self {
        if let Some(last) = self.actions.last_mut() {
            last.variant = Variant::Danger;
        }
        self
    }
}

/// The title of a dialog, or the app's row in its place, and the quiet cross at its end that
/// gives the dialog up.
fn head(title: &str, own: Option<AnyView>, open: Signal<bool>) -> Div {
    let cross = row().size(22.0, 22.0).shrink(0.0).align_items(Align::Center).justify(Justify::Center);
    let cross = cross.radius(token::RADIUS_MD).hover(|s| s.bg(token::PANEL_HOVER));
    let cross = base::close(cross.cursor(Cursor::Pointer), open);
    let cross = cross.child(icon(icons::X).size(14.0).color(token::TEXT_2));
    let headline = match own {
        Some(own) => column().grow(1.0).min_width(0.0).align_items(Align::Stretch).child(own),
        None => div().grow(1.0).font_weight(600).child(text(title.to_string()).size(size::TEXT_LG.size)),
    };
    let head = row().align_items(Align::Center).gap(4.0).padding_x(12.0).padding_top(8.0);
    head.padding_bottom(4.0).child(headline).child(cross)
}

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

/// A question with two answers: `yes` does `on_yes`, Cancel does nothing, and both close the
/// dialog.
pub fn confirm(
    open: Signal<bool>,
    title: impl Into<String>,
    message: impl Into<String>,
    yes: impl Into<String>,
    on_yes: impl Fn() + 'static,
) -> Dialog<Box<dyn Fn() -> AnyView>> {
    let message = message.into();
    let ask = dialog(
        open,
        title,
        Box::new(move || AnyView::new(text(message.clone()))) as Box<dyn Fn() -> AnyView>,
    );
    ask.action("Cancel", Variant::Secondary, || {}).action(yes, Variant::Primary, on_yes)
}

impl<V, F> View for Dialog<F>
where
    V: View + 'static,
    F: Fn() -> V + 'static,
{
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (open, title, body, actions) = (self.open, self.title, self.body, Rc::new(self.actions));
        let content = move || {
            let head = head(&title, None, open);
            let middle = div().padding_x(12.0).padding_y(8.0).text_color(token::TEXT_2).child(body());
            let page = base::page(column().width(440.0 - EDGE), title.clone());
            let page = page.child(head).child(middle);
            if actions.is_empty() {
                return page;
            }
            let mut foot = row().gap(4.0).padding(8.0).justify(Justify::End);
            foot = foot.border_top(1.0).border_color(token::LINE);
            for at in actions.iter() {
                let press = base::answer(open, at.act.clone());
                let face = button(at.label.clone()).variant(at.variant).on_click(press);
                let face = match at.key {
                    Some(key) => face.shortcut(key),
                    None => face,
                };
                foot = foot.child(match at.off.clone() {
                    Some(off) => face.disabled(move || off()),
                    None => face,
                });
            }
            page.child(foot)
        };
        popup(open, (Place::Top(80.0), 2), None, content).mount(cx)
    }
}
