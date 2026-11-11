//! The sections of the gallery's pages: each shows a group of the kit's components.

use std::collections::HashSet;
use std::rc::Rc;

use quadra::{Align, Div, NodeRef, Signal, column, div, row, signal, text};
use quadra_kit::{
    CellKind, Column, Date, MenuItem, Notice, Notices, Rule, Size, Status, TimeOfDay, Tone, TreeNode,
    Variant, accordion, avatar, badge, breadcrumb, button, calendar, card, card_head, checkbox, combobox,
    context_menu, date_picker, dialog, divider, field, heading, icon, icon_button, icons, input, kbd, list,
    menu, number_input, progress, progress_busy, radio_group, range_slider, segmented, select, size,
    skeleton, slider, spinner, split, status_label, switch, table, tabs, tag, textarea, time_input, token,
    tooltip, tree, validate,
};

/// A section: its title, then its rows.
fn section(title: &'static str) -> Div {
    let title = text(title).size(size::TEXT_SM.size).color(token::TEXT_2);
    column().gap(8.0).child(title)
}

fn line() -> Div {
    row().gap(8.0).align_items(Align::Center)
}

pub fn buttons() -> Div {
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

pub fn icons() -> Div {
    let all = [icons::CHECK, icons::CHEVRON_DOWN, icons::FOLDER, icons::FILE_TEXT, icons::BELL, icons::EYE];
    let sizes = [10.0, 12.0, 14.0, 16.0].map(|px| line().children(all.map(|body| icon(body).size(px))));
    let ok = icon(icons::CIRCLE_CHECK).size(16.0).color(token::OK);
    let wrong = icon(icons::CIRCLE_X).size(16.0).color(token::ERROR);
    section("Icons").child(line().gap(24.0).children(sizes)).child(line().child(ok).child(wrong))
}

pub fn inputs() -> Div {
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

pub fn numbers() -> Div {
    let (volume, count, zoom) = (signal(40.0f32), signal(3.0f64), signal(1.5f64));
    let (name, notes) = (
        signal("Al".to_string()),
        signal(
            "Several lines
of text"
                .to_string(),
        ),
    );
    let rules = [Rule::Required("A name is needed".into()), Rule::MinLen(3, "At least three letters".into())];
    let shown = text(move || format!("{:.0}", volume.get()));
    let sliders = line()
        .gap(16.0)
        .child(slider(volume).label("Volume"))
        .child(div().width(32.0).shrink(0.0).child(shown));
    let sliders = sliders.child(slider(signal(30.0)).disabled(true));
    let sliders = sliders.child(range_slider(signal(20.0), signal(70.0)).label("Price"));
    let steps = line().gap(16.0).child(number_input(count).range(0.0, 10.0));
    let steps = steps.child(number_input(zoom).step(0.25).decimals(2).stepper());
    let cities = ["H\u{E0} N\u{1ED9}i", "H\u{1EA3}i Ph\u{F2}ng", "Hu\u{1EBF}", "\u{110}\u{E0} N\u{1EB5}ng"];
    let steps = steps.child(combobox(cities, signal(String::new())).placeholder("City"));
    let named = field("Name", input(name)).help("As it shows to others").required();
    let named = named.error(move || validate(&name.get(), &rules));
    let fields =
        line().align_items(Align::Start).gap(16.0).child(named).child(field("Notes", textarea(notes, 3)));
    section("Numbers and fields").child(sliders).child(steps).child(fields)
}

pub fn dates() -> Div {
    let (day, time) = (signal(Date::new(2026, 11, 11)), signal(TimeOfDay::new(9, 30)));
    let fields = column().gap(8.0).child(date_picker(day)).child(time_input(time).minute_step(5));
    let fields = fields.child(text(move || format!("{} at {}", day.get(), time.get())).color(token::TEXT_2));
    section("Dates and times")
        .child(line().align_items(Align::Start).gap(16.0).child(calendar(day)).child(fields))
}

pub fn choices() -> Div {
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

pub fn overlays(all: Notices) -> Div {
    // `GALLERY_OPEN=menu`, `=dialog` or `=notice` shows that one at once: for a picture of it.
    let shown = std::env::var("GALLERY_OPEN").unwrap_or_default();
    let tell = move || {
        let card = Notice::new("Upload failed").body("The server did not answer.").tone(Tone::Danger);
        all.notify(card.action("Dismiss", || {}).primary("Retry", || {}));
    };
    let toast = move || {
        all.toast(Tone::Success, "Copied to the clipboard");
    };
    if shown == "notice" {
        tell();
        toast();
    }
    let (open, asking, opener) = (signal(shown == "menu"), signal(shown == "dialog"), NodeRef::new());
    let (wrap, size) = (signal(true), signal(1usize));
    let items = move || {
        let to = [MenuItem::new("Mail").icon(icons::MAIL), MenuItem::new("A link").icon(icons::LINK)];
        vec![
            MenuItem::heading("Edit"),
            MenuItem::new("Cut").icon(icons::TRASH_2).keys("Ctrl-X"),
            MenuItem::new("Copy").icon(icons::COPY).keys("Ctrl-C"),
            MenuItem::new("Paste").keys("Ctrl-V").disabled(true),
            MenuItem::new("Share").submenu(to),
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
    let controls = controls.child(button("Notify").on_click(tell)).child(button("Toast").on_click(toast));
    let other = div().padding(12.0).border(1.0).border_color(token::LINE).radius(size::RADIUS_LG);
    let other = other.child(text("A menu for the other button").color(token::TEXT_2));
    let other = context_menu(other, || vec![MenuItem::new("Rename"), MenuItem::new("Delete")]);
    let menus = menu(open, &opener, items);
    section("Overlays").child(controls).child(line().child(other)).child(menus).child(ask)
}

pub fn lists() -> Div {
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

pub fn parts() -> Div {
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
    let under_way = line().gap(16.0).child(progress(0.6)).child(progress_busy()).child(spinner());
    let under_way = under_way.child(skeleton(120.0, 16.0));
    section("Parts").child(heading("Heading", 2)).child(chips).child(under_way).child(boxes).child(framed)
}

pub fn navigation() -> Div {
    let all = [Status::Running, Status::Pending, Status::Failed, Status::Succeeded, Status::Terminating];
    let marks = line().gap(16.0).children(all.map(|status| status_label(status, "")));
    let way = breadcrumb(["workspace", "notes", "today.md"], |_| {});
    section("Status and the way").child(marks.child(status_label(Status::Unknown, "No word"))).child(way)
}

pub fn data() -> Div {
    // A thousand pods: what a table of a cluster shows.
    let pod = |i: usize| {
        let status = ["Running", "Running", "Pending", "Failed", "Succeeded"][i % 5];
        (format!("api-{:04}", i * 7 % 1000), (i * 37 % 100) as f64 / 10.0, status, 40 + 97 * i as i64)
    };
    let pods = Rc::new((0..1000).map(pod).collect::<Vec<_>>());
    let (count, chosen) = (pods.len(), signal(vec![2usize]));
    let columns = [
        Column::new("Name").width(220.0),
        Column::new("CPU").numeric().width(90.0),
        Column::new("Status").kind(CellKind::Status),
        Column::new("Age").kind(CellKind::Age).width(90.0),
    ];
    let cell = move |row: usize, column: usize| match (&pods[row], column) {
        (pod, 0) => pod.0.clone(),
        (pod, 1) => format!("{:.1}", pod.1),
        (pod, 2) => pod.2.to_string(),
        (pod, _) => pod.3.to_string(),
    };
    let pods = table(columns, move || count, cell).selected(chosen);
    let framed = div().height(340.0).border(1.0).border_color(token::LINE).child(pods);
    let picked = text(move || format!("{} selected", chosen.with(Vec::len))).color(token::TEXT_2);
    section("Table").child(framed).child(picked)
}

pub fn files() -> Div {
    let file = |name: &str| TreeNode::new(name, name).icon(icons::FILE_TEXT);
    let roots = move || {
        let kit = TreeNode::new("kit", "kit").icon(icons::FOLDER);
        let kit = kit.children(["button.rs", "menu.rs", "table.rs", "tree.rs"].map(file));
        let src = TreeNode::new("src", "src").icon(icons::FOLDER).children([kit, file("main.rs")]);
        let pages = (1..=200).map(|i| file(&format!("page-{i:03}.md")));
        vec![src, TreeNode::new("docs", "docs").icon(icons::FOLDER).children(pages), file("README.md")]
    };
    let (chosen, open) = (signal(Some("kit".to_string())), signal(HashSet::from(["src".to_string()])));
    let framed = div().size(300.0, 220.0).border(1.0).border_color(token::LINE);
    section("Tree").child(framed.child(tree(roots, chosen).open(open)))
}
