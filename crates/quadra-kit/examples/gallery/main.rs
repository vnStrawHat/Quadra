//! The kit's components in a real window, a page a group, in either theme:
//!
//!     tools/cargo.sh run -p quadra-kit --example gallery

mod pages;

use std::rc::Rc;

use quadra::{Signal, Theme, WindowOptions, column, div, dynamic, row, scroll, signal, use_context};
use quadra_kit::{
    Command, MenuItem, NavItem, Size, breadcrumb, button, command_palette, dark, icons, light, notices,
    sidebar, status_button, status_divider, status_item, statusbar, titlebar, toolbar,
};

use pages::{
    buttons, choices, data, dates, files, icons as icon_page, inputs, lists, navigation, numbers, overlays,
    parts,
};

/// The pages: what each is known by, its name in the sidebar, and its icon.
const PAGES: [(&str, &str, &str); 7] = [
    ("buttons", "Buttons", icons::SQUARE_CHECK),
    ("forms", "Forms", icons::SLIDERS_HORIZONTAL),
    ("overlays", "Overlays and lists", icons::LAYERS),
    ("parts", "Parts", icons::LAYOUT_GRID),
    ("navigation", "Navigation", icons::LIST_TREE),
    ("data", "Data", icons::TABLE),
    ("dates", "Dates", icons::CALENDAR),
];

fn main() -> quadra::Result<()> {
    let view = || {
        let theme = use_context::<Signal<Rc<Theme>>>().expect("an app gives its views the theme");
        let (is_dark, folded, all) = (signal(true), signal(false), notices().above(30.0));
        let swap = move || {
            is_dark.update(|dark| *dark = !*dark);
            theme.set(Rc::new(if is_dark.get_untracked() { dark() } else { light() }.theme()));
        };
        // `GALLERY_PAGE=2` starts on the third page: for a picture of it.
        let first = std::env::var("GALLERY_PAGE").ok().and_then(|page| page.parse().ok()).unwrap_or(0usize);
        let page = signal(PAGES[first.min(PAGES.len() - 1)].0);
        let shown = dynamic(move || match page.get() {
            "buttons" => column().gap(20.0).child(buttons()).child(icon_page()),
            "forms" => column().gap(20.0).child(inputs()).child(choices()).child(numbers()),
            "overlays" => column().gap(20.0).child(overlays(all)).child(lists()),
            "parts" => column().gap(20.0).child(parts()),
            "data" => column().gap(20.0).child(data()).child(files()),
            "dates" => column().gap(20.0).child(dates()),
            _ => column().gap(20.0).child(navigation()),
        });
        let name = move || PAGES.iter().find(|known| known.0 == page.get()).map_or("", |known| known.1);
        let places = sidebar(page).width(200.0).folded(move || folded.get());
        let places =
            places.group("Components", PAGES.map(|(id, name, body)| NavItem::new(id, name).icon(body)));
        let way = dynamic(move || breadcrumb(["Gallery", name()], move |_| page.set(PAGES[0].0)));
        let tools = toolbar().child(way).child(div().grow(1.0));
        let tools = tools.child(button("Other theme").size(Size::Sm).on_click(swap));
        let content = scroll(signal(0.0), column().padding(24.0).child(shown));
        let fold = status_button(icons::PANEL_LEFT, "Sidebar", move || !folded.get());
        let status = statusbar()
            .left(fold.on_click(move || folded.update(|folded| *folded = !*folded)))
            .left(status_divider())
            .left(status_item(None, move || {
                if is_dark.get() { "One Dark Pro" } else { "One Light" }.to_string()
            }))
            .right(status_item(Some(icons::CHECK), "Ready"));
        // What the app can do, found by typing: `GALLERY_OPEN=palette` shows it at once.
        let asking = signal(std::env::var("GALLERY_OPEN").is_ok_and(|shown| shown == "palette"));
        let commands = move || {
            let to = |(id, name, _): (&'static str, &str, &str)| {
                Command::new(format!("Go to {name}"), move || page.set(id))
            };
            let fold = Command::new("Toggle the sidebar", move || folded.update(|folded| *folded = !*folded));
            PAGES
                .into_iter()
                .map(to)
                .chain([fold.keys("ctrl-b"), Command::new("Other theme", swap)])
                .collect()
        };
        let ask = MenuItem::new("Command palette").keys("Ctrl-Shift-P").on_select(move || asking.set(true));
        let bar = titlebar("Quadra gallery").menu(move || vec![ask.clone(), MenuItem::new("About")]);
        let middle = row().grow(1.0).child(places).child(column().grow(1.0).child(tools).child(content));
        let over = command_palette(asking, commands);
        quadra_kit::root().child(bar).child(middle).child(status).child(all.host()).child(over)
    };
    let app = quadra_kit::install(quadra::app()?, &dark())?;
    app.window(WindowOptions::new("Quadra gallery").size(900.0, 640.0).custom_frame(), view)?.run()
}
