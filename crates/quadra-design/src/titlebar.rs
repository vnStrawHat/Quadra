//! The title bar of a window with its own frame (specs/behaviour/15, "Title bar"): the bar
//! the window is dragged by, with the app's menu and title at its left, the app's controls,
//! and the buttons that minimize, maximize and close at its right.

use std::rc::Rc;

use quadra_layout::{Align, Justify};
use quadra_platform::HitRegion;
use quadra_prim::Color;
use quadra_reactive::signal;
use quadra_tree::{NodeId, Role};
use quadra_view::{
    AnyView, Div, IntoProp, MountCx, NodeRef, Prop, View, by_text, div, dynamic, row, text, window,
};

use crate::button::icon_button;
use crate::icon::icon;
use crate::menu::{MenuItem, menu};
use crate::{icons, size, token};

/// The faces that have the glyphs of caption buttons: that of Windows 11, then that of 10.
const FACES: [&str; 2] = ["Segoe Fluent Icons", "Segoe MDL2 Assets"];
/// Minimize, maximize, restore and close in those faces, and drawn where there is neither.
const GLYPHS: [&str; 4] = ["\u{E921}", "\u{E922}", "\u{E923}", "\u{E8BB}"];
const DRAWN: [&str; 4] = [icons::MINUS, icons::WINDOW_MAXIMIZE, icons::WINDOW_RESTORE, icons::X];
/// What the close button turns under the pointer, on every theme.
const CLOSE: Color = Color::rgb(0xE81120);

type Items = Rc<dyn Fn() -> Vec<MenuItem>>;

pub struct Titlebar {
    title: Prop<String>,
    title_size: Option<f32>,
    icon: Option<&'static str>,
    menu: Option<Items>,
    controls: Vec<AnyView>,
}

/// A title bar that shows `title`: for a window opened with `WindowOptions::custom_frame`.
pub fn titlebar<M>(title: impl IntoProp<String, M>) -> Titlebar {
    Titlebar { title: title.into_prop(), title_size: None, icon: None, menu: None, controls: Vec::new() }
}

impl Titlebar {
    /// An icon before the title.
    pub fn icon(mut self, body: &'static str) -> Self {
        self.icon = Some(body);
        self
    }

    /// The app's menu, behind a button at the left end.
    pub fn menu(mut self, items: impl Fn() -> Vec<MenuItem> + 'static) -> Self {
        self.menu = Some(Rc::new(items));
        self
    }

    /// Something of the app's between the title and the window's buttons: a search field, a
    /// button. It works as anywhere else; the bar drags the window by the room left empty.
    pub fn control(mut self, view: impl View + 'static) -> Self {
        self.controls.push(AnyView::new(view));
        self
    }

    /// The title as the caption of a small window: this size, 12 px from the edge, in a bar
    /// of 34 px.
    pub fn title_size(mut self, px: f32) -> Self {
        self.title_size = Some(px);
        self
    }
}

/// A button of the window: 36 px wide, as tall as the bar, its glyph at 10 px.
fn caption(name: &'static str, height: f32, danger: bool) -> Div {
    let face = row().align_items(Align::Center).justify(Justify::Center).shrink(0.0).size(36.0, height);
    let face = face.role(Role::Button).label(name.to_string());
    match danger {
        true => face
            .hover(|s| s.bg(CLOSE).text_color(Color::WHITE))
            .active(|s| s.bg(CLOSE.alpha(0.8)).text_color(Color::WHITE)),
        false => face.hover(|s| s.bg(token::PANEL_HOVER)).active(|s| s.bg(token::PRESS)),
    }
}

impl View for Titlebar {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let height = if self.title_size.is_some() { 34.0 } else { 32.0 };
        let bar = row().align_items(Align::Center).shrink(0.0).height(by_text(height));
        let bar = bar.role(Role::Group).test_id("titlebar");
        let bar = bar.bg(token::CHROME).border_bottom(1.0).border_color(token::LINE_STRONG);
        let bar = bar.text_size(size::TEXT_SM.size).line_height(size::TEXT_SM.line_height());
        let mut bar = bar.window_region(HitRegion::Caption);
        if let Some(items) = self.menu {
            let (open, opener) = (signal(false), NodeRef::new());
            let button =
                icon_button(icons::MENU, "Menu").on_click(move || open.update(|open| *open = !*open));
            let button = div().padding_left(4.0).child(div().node_ref(&opener).child(button));
            bar = bar.child(button).child(menu(open, &opener, move || items()));
        }
        if let Some(body) = self.icon {
            bar = bar.child(div().padding_left(8.0).child(icon(body).size(14.0).color(token::TEXT_2)));
        }
        bar = bar.child(match self.title_size {
            Some(px) => div().padding_left(12.0).padding_right(8.0).text_size(px).child(text(self.title)),
            None => div().padding_x(8.0).child(text(self.title)),
        });
        bar = bar.child(row().align_items(Align::Center).gap(4.0).grow(1.0).children(self.controls));

        // The glyphs of the system's own caption buttons, where it has a face of them.
        let families = cx.tree.fonts().borrow_mut().text.backend().families();
        let face = FACES.into_iter().find(|face| families.iter().any(|family| family == face));
        let glyph = move |which: usize| match face {
            Some(face) => AnyView::new(div().font_family(face).text_size(10.0).child(text(GLYPHS[which]))),
            None => AnyView::new(icon(DRAWN[which]).size(12.0)),
        };
        let (window, name) = (window(), |maximized| if maximized { "Restore" } else { "Maximize" });
        let (min, max, close, is, shows) =
            (window.clone(), window.clone(), window.clone(), window.clone(), window);
        let minimize = caption("Minimize", height, false).on_click(move |_| min.minimize()).child(glyph(0));
        let maximize = caption("Maximize", height, false).window_region(HitRegion::Maximize);
        let maximize = maximize.label(move || name(is.is_maximized()).to_string());
        let maximize = maximize.on_click(move |_| max.toggle_maximize());
        let maximize = maximize.child(dynamic(move || glyph(1 + shows.is_maximized() as usize)));
        let close = caption("Close", height, true).on_click(move |_| close.close()).child(glyph(3));
        bar.child(minimize).child(maximize).child(close).mount(cx)
    }
}
