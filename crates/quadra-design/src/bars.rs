//! The bars of a window under its title (specs/behaviour/15, "Chrome"): the toolbar, the
//! status bar and what stands in it, and the breadcrumb.

use std::rc::Rc;

use quadra_layout::Align;
use quadra_platform::Cursor;
use quadra_tree::{NodeId, Role};
use quadra_ui::{AnyView, Div, IntoProp, MountCx, View, by_text, div, row, text};

use crate::button::{Button, icon_button};
use crate::icon::icon;
use crate::{fonts, size, token};

/// A toolbar: 45 px on the canvas with a line under it, its controls 4 px apart. Give it
/// children; [`crate::vdivider`] splits groups.
pub fn toolbar() -> Div {
    let bar = row().align_items(Align::Center).gap(4.0).padding_x(8.0).padding_y(6.0).height(by_text(45.0));
    bar.shrink(0.0).bg(token::CANVAS).border_bottom(1.0).border_color(token::LINE)
}

#[derive(Default)]
pub struct Statusbar {
    left: Vec<AnyView>,
    right: Vec<AnyView>,
}

/// The strip at the bottom of a window: 30 px, 12 px text, what it holds at its left end and
/// at its right.
pub fn statusbar() -> Statusbar {
    Statusbar::default()
}

impl Statusbar {
    pub fn left(mut self, view: impl View + 'static) -> Self {
        self.left.push(AnyView::new(view));
        self
    }

    pub fn right(mut self, view: impl View + 'static) -> Self {
        self.right.push(AnyView::new(view));
        self
    }
}

impl View for Statusbar {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let bar = row().align_items(Align::Center).gap(12.0).padding_x(8.0).height(by_text(30.0)).shrink(0.0);
        let bar = bar.bg(token::STATUS_BG).border_top(1.0).border_color(token::LINE_STRONG);
        let bar = bar.text_size(size::TEXT_SM.size).line_height(size::TEXT_SM.line_height());
        let bar = bar.role(Role::Group).test_id("statusbar");
        bar.children(self.left).child(div().grow(1.0)).children(self.right).mount(cx)
    }
}

/// An entry of a status bar: words, after an icon if there is one.
pub fn status_item<M>(body: Option<&'static str>, words: impl IntoProp<String, M>) -> Div {
    let item = row().align_items(Align::Center).gap(4.0);
    let item = match body {
        Some(body) => item.child(icon(body).size(14.0).color(token::TEXT_2)),
        None => item,
    };
    item.child(text(words))
}

/// A toggle of a status bar that is only an icon: muted, and the accent while `on`: the
/// panel it stands for is open.
pub fn status_button<M>(body: &'static str, name: &'static str, on: impl IntoProp<bool, M>) -> Button {
    icon_button(body, name).muted().toggled(on)
}

/// What splits groups in a status bar: a line of 1 by 14 px.
pub fn status_divider() -> Div {
    div().size(1.0, 14.0).shrink(0.0).bg(token::LINE_STRONG)
}

pub struct Breadcrumb {
    parts: Vec<String>,
    pick: Rc<dyn Fn(usize)>,
}

/// The way to where the user is: mono, muted names with a mark between them. `pick` is given
/// the index of a name that is clicked; the last one is where the user is, and is not a link.
pub fn breadcrumb<S: Into<String>>(
    parts: impl IntoIterator<Item = S>,
    pick: impl Fn(usize) + 'static,
) -> Breadcrumb {
    Breadcrumb { parts: parts.into_iter().map(Into::into).collect(), pick: Rc::new(pick) }
}

impl View for Breadcrumb {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let mut way = row().align_items(Align::Center).gap(4.0).font_family(fonts::MONO);
        way = way.text_color(token::TEXT_2);
        let last = self.parts.len().saturating_sub(1);
        for (i, part) in self.parts.into_iter().enumerate() {
            if i > 0 {
                way = way.child(text("\u{203A}").color(token::TEXT_3));
            }
            let crumb = div().child(text(part));
            way = way.child(match i < last {
                true => {
                    let pick = self.pick.clone();
                    let link = crumb.role(Role::Link).cursor(Cursor::Pointer);
                    link.hover(|s| s.text_color(token::TEXT)).on_click(move |_| pick(i))
                }
                false => crumb,
            });
        }
        way.mount(cx)
    }
}
