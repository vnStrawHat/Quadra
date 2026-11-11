//! The kit's components in a real window, a section a component, in either theme:
//!
//!     tools/cargo.sh run -p quadra-kit --example gallery

use std::rc::Rc;

use quadra::{
    Align, Div, NodeRef, Signal, Theme, WindowOptions, column, div, row, signal, text, use_context,
};
use quadra_kit::{
    MenuItem, Size, Variant, button, checkbox, dark, icon, icon_button, icons, input, light, menu,
    radio_group, segmented, select, size, switch, token,
};

/// A section: its title, then its rows.
fn section(title: &'static str) -> Div {
    let title = text(title).size(size::TEXT_SM.size).color(token::TEXT_2);
    column().gap(8.0).child(title)
}

fn line() -> Div {
    row().gap(8.0).align_items(Align::Center)
}

fn buttons() -> Div {
    let wrap = signal(true);
    let variants = [
        ("Primary", Variant::Primary),
        ("Secondary", Variant::Secondary),
        ("Outline", Variant::Outline),
        ("Ghost", Variant::Ghost),
        ("Danger", Variant::Danger),
        ("Link", Variant::Link),
    ];
    let sizes = [("Extra small", Size::Xs), ("Small", Size::Sm), ("Medium", Size::Md), ("Large", Size::Lg)];
    let by_variant = line().children(variants.map(|(name, variant)| button(name).variant(variant)));
    let by_size = line().children(sizes.map(|(name, size)| button(name).size(size)));
    let off = line().children(variants.map(|(name, variant)| button(name).variant(variant).disabled(true)));
    let toggle = button("Word wrap").variant(Variant::Ghost).toggled(wrap);
    let toggle = toggle.on_click(move || wrap.update(|on| *on = !*on));
    let states = line().child(toggle).child(text(move || if wrap.get() { "on" } else { "off" }));
    let with_icons = line()
        .child(button("Save").variant(Variant::Primary).icon(icons::SAVE))
        .child(button("Search").icon(icons::SEARCH))
        .child(button("Delete").variant(Variant::Danger).size(Size::Sm).icon(icons::TRASH_2))
        .child(icon_button(icons::SETTINGS, "Settings"))
        .child(icon_button(icons::X, "Close"));
    section("Buttons").child(by_variant).child(by_size).child(off).child(states).child(with_icons)
}

fn icons() -> Div {
    let all = [icons::CHECK, icons::CHEVRON_DOWN, icons::FOLDER, icons::FILE_TEXT, icons::BELL, icons::EYE];
    let sizes = [10.0, 12.0, 14.0, 16.0].map(|px| line().children(all.map(|body| icon(body).size(px))));
    let ok = icon(icons::CIRCLE_CHECK).size(16.0).color(token::OK);
    let wrong = icon(icons::CIRCLE_X).size(16.0).color(token::ERROR);
    section("Icons").child(line().gap(24.0).children(sizes)).child(line().child(ok).child(wrong))
}

fn inputs() -> Div {
    let (name, query, secret) =
        (signal("Quadra".to_string()), signal(String::new()), signal("secret".to_string()));
    let wrong = signal("not a number".to_string());
    let first = line()
        .child(input(name).placeholder("Name"))
        .child(input(query).placeholder("Search").icon(icons::SEARCH));
    let second = line()
        .child(input(secret).password().placeholder("Password").width(160.0))
        .child(input(wrong).invalid(true).width(160.0))
        .child(input(signal(String::new())).placeholder("Disabled").disabled(true).width(160.0));
    section("Inputs").child(first).child(second)
}

fn choices() -> Div {
    let (wrap, numbers, live, quiet) = (signal(true), signal(false), signal(true), signal(false));
    let (mode, view, density) = (signal(1usize), signal(0usize), signal(1usize));
    let boxes = line()
        .gap(16.0)
        .child(checkbox(wrap).label("Word wrap"))
        .child(checkbox(numbers).label("Line numbers"))
        .child(checkbox(signal(true)).label("Disabled").disabled(true))
        .child(switch(live).label("Live"))
        .child(switch(quiet).label("Quiet"));
    let one_of = line()
        .gap(24.0)
        .child(radio_group(["Light", "Dark", "System"], mode).horizontal())
        .child(segmented(["List", "Grid", "Board"], view))
        .child(segmented(["Compact", "Comfortable"], density).size(Size::Md));
    section("Selection").child(boxes).child(one_of)
}

fn overlays() -> Div {
    // `GALLERY_OPEN=1` shows the menu at once: for a picture of it.
    let (open, opener) = (signal(std::env::var_os("GALLERY_OPEN").is_some()), NodeRef::new());
    let (wrap, size) = (signal(true), signal(1usize));
    let items = move || {
        vec![
            MenuItem::new("Cut").icon(icons::TRASH_2).keys("Ctrl-X"),
            MenuItem::new("Copy").icon(icons::COPY).keys("Ctrl-C"),
            MenuItem::new("Paste").keys("Ctrl-V").disabled(true),
            MenuItem::separator(),
            MenuItem::new("Word wrap")
                .checked(wrap.get_untracked())
                .on_select(move || wrap.update(|on| *on = !*on)),
        ]
    };
    let edit = button("Edit").icon(icons::CHEVRON_DOWN).on_click(move || open.update(|open| *open = !*open));
    let edit = div().node_ref(&opener).child(edit);
    let sizes = select(["Small", "Medium", "Large"], size);
    section("Overlays").child(line().child(edit).child(sizes)).child(menu(open, &opener, items))
}

fn main() -> quadra::Result<()> {
    let view = || {
        let theme = use_context::<Signal<Rc<Theme>>>().expect("an app gives its views the theme");
        let is_dark = signal(true);
        let swap = move || {
            is_dark.update(|dark| *dark = !*dark);
            theme.set(Rc::new(if is_dark.get_untracked() { dark() } else { light() }.theme()));
        };
        let name = text(move || if is_dark.get() { "One Dark Pro" } else { "One Light" });
        let name = name.size(size::TEXT_LG.size).weight(size::WEIGHT_STRONG);
        let head = line().child(name).child(button("Other theme").size(Size::Sm).on_click(swap));
        quadra_kit::root()
            .padding(24.0)
            .gap(20.0)
            .child(head)
            .child(buttons())
            .child(icons())
            .child(inputs())
            .child(choices())
            .child(overlays())
    };
    let app = quadra_kit::install(quadra::app()?, &dark())?;
    app.window(WindowOptions::new("Quadra gallery").size(680.0, 860.0), view)?.run()
}
