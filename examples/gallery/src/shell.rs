//! The gallery's window: its title bar, the pages in a sidebar, the page that shows, and a
//! status bar.

use std::rc::Rc;

use quadra::{
    Action, Display, Signal, TextSize, Theme, View, column, div, dynamic, row, scroll, signal, use_context,
};
use quadra_design::{
    Command, MenuItem, NavItem, Notices, Status, Tone, command_palette, dark, icon_button, icons, light,
    notices, sidebar, status_button, status_divider, status_item, status_label_sized, statusbar, titlebar,
    tooltip,
};

use crate::registry::{PAGES, make};

/// Opens the command palette: bound to Ctrl-K by the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenPalette;

impl Action for OpenPalette {
    const NAME: &'static str = "gallery::open_palette";
}

/// The whole window a step larger, a step smaller, or as it was (`0`): Ctrl-=, Ctrl-minus,
/// Ctrl-0 (specs/12, "Text size").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Zoom(pub i8);

impl Action for Zoom {
    const NAME: &'static str = "gallery::zoom";
}

/// What the gallery shows: the page, and the theme. Made where the window's view is built;
/// a copy is the same one, so a tool can turn its pages from outside.
#[derive(Clone, Copy)]
pub struct Gallery {
    pub page: Signal<&'static str>,
    pub dark: Signal<bool>,
    /// What the pages tell the user: above the status bar.
    pub notices: Notices,
    /// Whether the command palette is open.
    pub palette: Signal<bool>,
}

impl Gallery {
    pub fn new(page: &'static str, dark: bool) -> Gallery {
        let notices = notices().above(32.0);
        Gallery { page: signal(page), dark: signal(dark), notices, palette: signal(false) }
    }

    /// What the palette offers: a few commands of an app, and every page.
    fn commands(self) -> Vec<Command> {
        let says = move |tone, words: &'static str| {
            move || {
                self.notices.toast(tone, words);
            }
        };
        vec![
            Command::new("Mở tệp…", says(Tone::Info, "Mở tệp")).keys("ctrl-o"),
            Command::new("Lưu", says(Tone::Success, "Đã lưu")).keys("ctrl-s"),
            Command::new("Đổi giao diện sáng/tối", move || self.dark.update(|dark| *dark = !*dark)),
        ]
    }

    pub fn view(self) -> impl View {
        let Gallery { page, dark, notices, palette: asking } = self;
        // The theme of the window follows `dark` once it is turned: until then it is the one
        // the app was given (the pictures of high contrast keep theirs).
        let theme = use_context::<Signal<Rc<Theme>>>().expect("an app gives its views the theme");
        let name = move || if dark.get() { "One Dark Pro" } else { "One Light" };
        let first = std::cell::Cell::new(true);
        quadra::effect(move || {
            let theme_of = if dark.get() { dark_theme() } else { light_theme() };
            if !first.replace(false) {
                theme.set(Rc::new(theme_of));
            }
        });
        let hidden = signal(false);

        let toggle = dynamic(move || {
            let body = if dark.get() { icons::SUN } else { icons::MOON };
            let button =
                icon_button(body, "Toggle theme").on_click(move || dark.update(|dark| *dark = !*dark));
            tooltip("Toggle theme", button)
        });
        let menu = move || {
            let ask = MenuItem::new("Command palette").keys("Ctrl-K").on_select(move || asking.set(true));
            vec![ask, MenuItem::separator(), MenuItem::new("About Quadra")]
        };
        let head = titlebar("Quadra Gallery").menu(menu).control(div().grow(1.0)).control(toggle);

        let mut places = sidebar(page).search(signal(String::new()));
        let mut groups: Vec<&str> = Vec::new();
        for known in PAGES {
            if !groups.contains(&known.group) {
                groups.push(known.group);
            }
        }
        for group in groups {
            let of = PAGES.iter().filter(|known| known.group == group);
            places =
                places.group(group, of.map(|known| NavItem::new(known.id, known.title).icon(known.icon)));
        }
        let places =
            div().display(move || if hidden.get() { Display::None } else { Display::Flex }).child(places);
        // Each page is built when it is gone to, and scrolls from its top.
        let shown = dynamic(move || scroll(signal(0.0), make(page.get(), self)));
        let middle = row().grow(1.0).child(places).child(shown.container(column().grow(1.0)));

        let title = move || PAGES.iter().find(|known| known.id == page.get()).map_or("", |known| known.title);
        let pages = status_button(icons::PANEL_LEFT, "Pages", move || !hidden.get());
        let pages = pages.on_click(move || hidden.update(|hidden| *hidden = !*hidden));
        let search = status_button(icons::SEARCH, "Commands", false).on_click(move || asking.set(true));
        let status = statusbar()
            .left(tooltip("Pages", pages))
            .left(tooltip("Commands", search).keys("Ctrl-K"))
            .left(status_divider())
            .left(status_label_sized(Status::Running, "Ready", 12.0))
            .left(status_item(Some(icons::LAYERS), move || title().to_string()))
            .right(status_item(None, format!("{} pages", PAGES.len())))
            .right(status_item(Some(icons::PALETTE), move || format!("{} \u{B7} compact", name())))
            .right(status_item(Some(icons::COMMAND), "Ctrl-K"));
        let palette = command_palette(asking, move || self.commands());
        let all = quadra_design::root().on_action::<OpenPalette>(move |_| asking.set(true));
        let size = use_context::<Signal<TextSize>>(); // none in a test without an app
        let all = all.on_action::<Zoom>(move |step| {
            let to = |zoom: f32| if step.0 == 0 { 1.0 } else { (zoom + 0.1 * step.0 as f32).clamp(0.5, 3.0) };
            if let Some(size) = size {
                size.update(|size| size.zoom = to(size.zoom));
            }
        });
        all.child(head).child(middle).child(status).child(palette).child(notices.host())
    }
}

fn dark_theme() -> Theme {
    dark().theme()
}

fn light_theme() -> Theme {
    light().theme()
}
