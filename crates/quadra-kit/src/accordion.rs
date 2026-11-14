//! The accordion (specs/behaviour/09, "Layout"): sections under one another, each opened and
//! closed by its head. Whether a section is open is a signal of the caller's.

use quadra_layout::Align;
use quadra_platform::Cursor;
use quadra_reactive::Signal;
use quadra_tree::{NodeId, Role};
use quadra_ui::{AnyView, Div, MountCx, View, column, div, dynamic, row, show, text};

use crate::action::{CONTROL, Confirm};
use crate::icon::icon;
use crate::{icons, size, token};

type Body = Box<dyn Fn() -> AnyView>;

#[derive(Default)]
pub struct Accordion {
    items: Vec<(String, Signal<bool>, Body)>,
}

/// An accordion with no sections yet: add them with [`Accordion::item`].
pub fn accordion() -> Accordion {
    Accordion::default()
}

impl Accordion {
    /// A section: its title, the signal that says whether it is open, and what builds its
    /// body each time it opens.
    pub fn item<V: View + 'static>(
        mut self,
        title: impl Into<String>,
        open: Signal<bool>,
        body: impl Fn() -> V + 'static,
    ) -> Self {
        self.items.push((title.into(), open, Box::new(move || AnyView::new(body()))));
        self
    }
}

/// The head of a section: a chevron, right when closed and down when open, then the title.
/// A click, Enter and Space open and close it.
fn head(title: String, open: Signal<bool>) -> Div {
    let toggle = move || open.update(|open| *open = !*open);
    let chevron = move || {
        let body = if open.get() { icons::CHEVRON_DOWN } else { icons::CHEVRON_RIGHT };
        icon(body).size(14.0).color(token::TEXT_2)
    };
    let head = row().align_items(Align::Center).gap(4.0).padding_x(8.0).height(size::H_MD);
    let head = head.cursor(Cursor::Pointer).role(Role::Button).expanded(move || open.get());
    let head = head.focusable().key_context(CONTROL).on_action::<Confirm>(move |_| toggle());
    let head = head.hover(|s| s.bg(token::PANEL_HOVER)).focus_visible(|s| s.bg(token::PANEL_HOVER));
    head.on_click(move |_| toggle()).child(dynamic(chevron)).child(text(title))
}

impl View for Accordion {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let mut all = column().border(1.0).border_color(token::LINE_STRONG).radius(size::RADIUS_MD);
        for (i, (title, open, body)) in self.items.into_iter().enumerate() {
            let head = head(title, open);
            // A line between a section and the one above it.
            let head = if i > 0 { head.border_top(1.0).border_color(token::LINE) } else { head };
            let inside =
                move || div().padding_left(26.0).padding_right(12.0).padding_bottom(12.0).child(body());
            all = all.child(head).child(show(move || open.get(), inside));
        }
        all.mount(cx)
    }
}
