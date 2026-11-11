//! The kit's components in a real window, a section a component, in either theme:
//!
//!     tools/cargo.sh run -p quadra-kit --example gallery

use std::rc::Rc;

use quadra::{
    Align, Div, NodeRef, Signal, Theme, WindowOptions, column, div, dynamic, row, signal, text, use_context,
};
use quadra_kit::{
    MenuItem, Size, Tone, Variant, accordion, avatar, badge, button, card, card_head, checkbox, dark, dialog,
    divider, heading, icon, icon_button, icons, input, kbd, light, list, menu, radio_group, segmented,
    select, size, split, switch, tabs, tag, titlebar, token, tooltip,
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
    // `GALLERY_OPEN=menu` or `=dialog` shows that one at once: for a picture of it.
    let shown = std::env::var("GALLERY_OPEN").unwrap_or_default();
    let (open, asking, opener) = (signal(shown == "menu"), signal(shown == "dialog"), NodeRef::new());
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
    let save = tooltip("Save the file", icon_button(icons::SAVE, "Save")).keys("Ctrl-S");
    let delete = button("Delete").variant(Variant::Danger).on_click(move || asking.set(true));
    let ask = dialog(asking, "Delete the file?", || text("It cannot be brought back."))
        .action("Cancel", Variant::Ghost, || {})
        .action("Delete", Variant::Danger, || {});
    let controls = line().child(edit).child(sizes).child(save).child(delete);
    section("Overlays").child(controls).child(menu(open, &opener, items)).child(ask)
}

fn lists() -> Div {
    let (page, view, file) = (signal(0usize), signal(0usize), signal(Some(2usize)));
    let pages = line()
        .gap(24.0)
        .child(tabs(["Files", "Search", "History"], page))
        .child(tabs(["List", "Grid"], view).pill());
    let name = |i: Signal<usize>| text(move || format!("notes-{:03}.md", i.get()));
    let files = list(|| 200, file, name).empty("No files");
    let framed = div().size(240.0, 117.0).border(1.0).border_color(token::LINE).child(files);
    let chosen = text(move || file.get().map_or("none".to_string(), |i| format!("row {i}")));
    section("Tabs and lists").child(pages).child(line().align_items(Align::Start).child(framed).child(chosen))
}

fn parts() -> Div {
    let (general, advanced, share) = (signal(true), signal(false), signal(0.4f32));
    let chips = line()
        .child(badge("neutral", Tone::Neutral))
        .child(badge("primary", Tone::Primary))
        .child(badge("ok", Tone::Success))
        .child(badge("warning", Tone::Warning))
        .child(badge("error", Tone::Danger))
        .child(badge("info", Tone::Info))
        .child(tag("v1.2"))
        .child(kbd("ctrl-shift-p"))
        .child(avatar("Ada Lovelace", 24.0));
    let about = card().width(260.0).child(card_head("Card", "What belongs together")).child(divider());
    let about = about.child(text("A bordered box on the panel's plane."));
    let sections =
        accordion()
            .item("General", general, || text("Name and language"))
            .item("Advanced", advanced, || text("Proxy and certificates"));
    let panes =
        split(div().padding(8.0).child(text("First")), div().padding(8.0).child(text("Second")), share);
    let framed = div().size(330.0, 90.0).border(1.0).border_color(token::LINE).child(panes);
    let boxes =
        line().align_items(Align::Start).gap(16.0).child(about).child(column().width(330.0).child(sections));
    section("Parts").child(heading("Heading", 2)).child(chips).child(boxes).child(framed)
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
        // `GALLERY_PAGE=2` starts on the third page: for a picture of it.
        let first = std::env::var("GALLERY_PAGE").ok().and_then(|page| page.parse().ok());
        let page = signal(first.unwrap_or(0usize));
        let shown = dynamic(move || match page.get() {
            0 => column().gap(20.0).child(buttons()).child(icons()),
            1 => column().gap(20.0).child(inputs()).child(choices()),
            3 => column().gap(20.0).child(parts()),
            _ => column().gap(20.0).child(overlays()).child(lists()),
        });
        let pages = tabs(["Buttons", "Forms", "Overlays and lists", "Parts"], page);
        let page = column().grow(1.0).padding(24.0).gap(20.0).child(head).child(pages).child(shown);
        let bar = titlebar("Quadra gallery").menu(|| vec![MenuItem::new("About"), MenuItem::new("Quit")]);
        quadra_kit::root().child(bar).child(page)
    };
    let app = quadra_kit::install(quadra::app()?, &dark())?;
    app.window(WindowOptions::new("Quadra gallery").size(680.0, 600.0).custom_frame(), view)?.run()
}
