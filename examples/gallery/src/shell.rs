//! The gallery's window: its title bar, the pages in a sidebar, the page that shows, and a
//! status bar.

use std::rc::Rc;

use quadra::{Action, Display, Signal, Theme, View, column, div, dynamic, row, scroll, signal, use_context};
use quadra_kit::{
    Command, MenuItem, NavItem, Notices, Status, command_palette, dark, icon_button, icons, light, notices,
    sidebar, status_button, status_divider, status_item, status_label, statusbar, titlebar, tooltip,
};

use crate::registry::{PAGES, make};

/// Opens the command palette: bound to Ctrl-K by the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenPalette;

impl Action for OpenPalette {
    const NAME: &'static str = "gallery::open_palette";
}

/// What the gallery shows: the page, and the theme. Made where the window's view is built;
/// a copy is the same one, so a tool can turn its pages from outside.
#[derive(Clone, Copy)]
pub struct Gallery {
    pub page: Signal<&'static str>,
    pub dark: Signal<bool>,
    /// What the pages tell the user: above the status bar.
    pub notices: Notices,
}

impl Gallery {
    pub fn new(page: &'static str, dark: bool) -> Gallery {
        Gallery { page: signal(page), dark: signal(dark), notices: notices().above(30.0) }
    }

    /// What the palette offers: every page, and the other theme.
    fn commands(self) -> Vec<Command> {
        let go = |page: &'static crate::PageInfo| {
            Command::new(format!("Go to {}", page.title), move || self.page.set(page.id))
        };
        let theme = Command::new("Toggle theme", move || self.dark.update(|dark| *dark = !*dark));
        PAGES.iter().map(go).chain([theme]).collect()
    }

    pub fn view(self) -> impl View {
        let Gallery { page, dark, notices } = self;
        // The theme of the window follows `dark`.
        let theme = use_context::<Signal<Rc<Theme>>>().expect("an app gives its views the theme");
        let name = move || if dark.get() { "One Dark Pro" } else { "One Light" };
        quadra::effect(move || theme.set(Rc::new(if dark.get() { dark_theme() } else { light_theme() })));
        let (hidden, asking) = (signal(false), signal(false));

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

        let mut places = sidebar(page);
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
            .left(status_label(Status::Running, "Ready"))
            .left(status_item(Some(icons::LAYERS), move || title().to_string()))
            .right(status_item(None, format!("{} pages", PAGES.len())))
            .right(status_item(Some(icons::PALETTE), move || format!("{} \u{B7} compact", name())))
            .right(status_item(Some(icons::COMMAND), "Ctrl-K"));
        let palette = command_palette(asking, move || self.commands());
        let all = quadra_kit::root().on_action::<OpenPalette>(move |_| asking.set(true));
        all.child(head).child(middle).child(status).child(palette).child(notices.host())
    }
}

fn dark_theme() -> Theme {
    dark().theme()
}

fn light_theme() -> Theme {
    light().theme()
}
