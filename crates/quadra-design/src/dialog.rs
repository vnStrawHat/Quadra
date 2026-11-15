//! The dialog (specs/behaviour/15, "Dialog"): a question the user answers before going on,
//! and the drawer, which is one along an edge of the window.
//! It hangs 80 px from the top of the window, centred, with no backdrop: the cross at the end
//! of its title, Escape or a press outside gives it up.

use std::rc::Rc;

use quadra_headless::dialog as base;
use quadra_layout::{Align, Justify};
use quadra_platform::Cursor;
use quadra_prim::{Ease, Point};
use quadra_reactive::Signal;
use quadra_tree::NodeId;
use quadra_view::{Div, MountCx, Place, View, column, div, row, text, timeline};

use crate::button::{Variant, button};
use crate::icon::icon;
use crate::popup::{EDGE, Edge, popup};
use crate::{icons, size, token};

type Act = Rc<dyn Fn()>;

pub struct Dialog<F> {
    open: Signal<bool>,
    title: String,
    body: F,
    actions: Vec<(String, Variant, Act)>,
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
        self.actions.push((label.into(), variant, Rc::new(f)));
        self
    }
}

/// The title of a dialog, and the quiet cross at its end that gives the dialog up.
fn head(title: &str, open: Signal<bool>) -> Div {
    let cross = row().size(22.0, 22.0).align_items(Align::Center).justify(Justify::Center);
    let cross = cross.radius(token::RADIUS_MD).hover(|s| s.bg(token::PANEL_HOVER));
    let cross = base::close(cross.cursor(Cursor::Pointer), open);
    let cross = cross.child(icon(icons::X).size(14.0).color(token::TEXT_2));
    let headline = div().grow(1.0).child(text(title.to_string()).size(size::TEXT_LG.size));
    let head = row().align_items(Align::Center).gap(4.0).padding_x(12.0).padding_top(8.0);
    head.padding_bottom(4.0).child(headline).child(cross)
}

pub struct Drawer<F> {
    open: Signal<bool>,
    edge: Edge,
    title: String,
    size: f32,
    body: F,
}

/// A dialog that lies along an edge of the window, over all of its length, while `open` is
/// true: a title over what `body` builds. It reaches 360 px into the window.
pub fn drawer<V, F>(open: Signal<bool>, edge: Edge, title: impl Into<String>, body: F) -> Drawer<F>
where
    V: View + 'static,
    F: Fn() -> V + 'static,
{
    Drawer { open, edge, title: title.into(), size: 360.0, body }
}

impl<F> Drawer<F> {
    /// How far the drawer reaches into the window, with its border.
    pub fn size(mut self, px: f32) -> Self {
        self.size = px;
        self
    }
}

impl<V, F> View for Drawer<F>
where
    V: View + 'static,
    F: Fn() -> V + 'static,
{
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let Drawer { open, edge, title, size, body } = self;
        let content = move || {
            let page = base::page(column().grow(1.0), title.clone());
            let page = match edge {
                Edge::Bottom => page.height(size - EDGE),
                _ => page.width(size - EDGE),
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
            page.child(head(&title, open)).child(middle.child(body()))
        };
        popup(open, edge, None, content).mount(cx)
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
) -> impl View {
    let message = message.into();
    let ask = dialog(open, title, move || text(message.clone()));
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
            let head = head(&title, open);
            let middle = div().padding_x(12.0).padding_y(8.0).text_color(token::TEXT_2).child(body());
            let page = base::page(column().width(440.0 - EDGE), title.clone());
            let page = page.child(head).child(middle);
            if actions.is_empty() {
                return page;
            }
            let mut foot = row().gap(4.0).padding(8.0).justify(Justify::End);
            foot = foot.border_top(1.0).border_color(token::LINE);
            for (label, variant, act) in actions.iter() {
                let press = base::answer(open, act.clone());
                foot = foot.child(button(label.clone()).variant(*variant).on_click(press));
            }
            page.child(foot)
        };
        popup(open, (Place::Top(80.0), 2), None, content).mount(cx)
    }
}
