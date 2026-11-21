//! What takes the user somewhere: the sidebar, the toolbar, the breadcrumb, the menu bar, the
//! status bar and the command palette.

use quadra::Token;
use quadra::{Div, column, div, dynamic, row, signal};
use quadra_design::{
    Command, MenuItem, NavItem, Size, Status, breadcrumb, button, command_palette, icon, icon_button, icons,
    kbd, menu_button, menubar, sidebar, status_item, status_label, statusbar, token, toolbar, tooltip,
    vdivider,
};

use crate::common::{demo, framed, hint, line, page};
use crate::shell::Gallery;

pub fn sidebar_page() -> Div {
    let place = signal("pods");
    let places = sidebar(place)
        .head(
            row()
                .align_items(quadra::Align::Center)
                .child(quadra::text("prod-cluster"))
                .child(div().grow(1.0))
                .child(icon_button(icons::REFRESH_CW, "Refresh").size(Size::Xs)),
        )
        .group(
            "Workloads",
            [
                NavItem::new("pods", "Pods").icon(icons::LAYERS),
                NavItem::new("deploy", "Deployments").icon(icons::REFRESH_CW),
                NavItem::new("jobs", "Jobs").icon(icons::CLOCK),
            ],
        )
        .group(
            "Config",
            [
                NavItem::new("maps", "Config maps").icon(icons::FILE),
                NavItem::new("secrets", "Secrets").icon(icons::LOCK),
            ],
        )
        .counts(|id| match id {
            "pods" => Some(420),
            "deploy" => Some(96),
            "jobs" => Some(12),
            "secrets" => Some(148),
            _ => None,
        })
        .foot(
            quadra_design::input(signal(String::new()))
                .small()
                .fill()
                .icon(icons::SEARCH)
                .placeholder("Filter by pod name"),
        );
    let said = div().grow(1.0).padding(16.0).child(hint(move || format!("Selected: {}", place.get())));
    page(
        "Sidebar",
        "Sections of places. The current place is two tones up, the hovered one a tone.",
        [demo(
            "Cluster navigation",
            framed(row().height(300.0).child(places).child(said)),
            "sidebar(place).group(\"Workloads\", [NavItem::new(\"pods\", \"Pods\")]).counts(|id| ..).foot(filter)",
        )],
    )
}

pub fn toolbar_page() -> Div {
    let tool = |body, name: &'static str| tooltip(name, icon_button(body, name).size(Size::Md));
    let bar = toolbar()
        .child(tool(icons::BOLD, "Bold"))
        .child(tool(icons::ITALIC, "Italic"))
        .child(vdivider())
        .child(tool(icons::LIST, "List"))
        .child(tool(icons::IMAGE, "Image"))
        .child(div().grow(1.0))
        .child(button("Save").icon(icons::SAVE).size(Size::Sm));
    page(
        "Toolbar",
        "A strip of related actions above the content it acts on.",
        [demo("Editor", framed(bar), "toolbar().child(icon_button(..)).child(vdivider())")],
    )
}

pub fn breadcrumb_page() -> Div {
    let crumb = signal(3usize);
    let parts = ["cluster-prod", "default", "Deployments", "api"];
    let way = dynamic(move || breadcrumb(parts[..=crumb.get().min(3)].to_vec(), move |i| crumb.set(i)));
    page(
        "Breadcrumb",
        "Where the view sits in a hierarchy. Every part but the last is a link.",
        [
            demo("Resource path", way, "breadcrumb([\"cluster\", \"default\", \"api\"], |i| ..)"),
            demo(
                "In a narrow bar",
                div()
                    .width(340.0)
                    .min_width(0.0)
                    .child(breadcrumb(["cluster-prod", "Workloads", "Deployments", "api-gateway"], |_| ())),
                "one line: the middle parts are cut first, the first and the last kept",
            ),
        ],
    )
}

pub fn menubar_page() -> Div {
    let file = || {
        vec![
            MenuItem::new("New file").keys("Ctrl-N"),
            MenuItem::new("Open…").keys("Ctrl-O"),
            MenuItem::separator(),
            MenuItem::new("Exit"),
        ]
    };
    let edit = || vec![MenuItem::new("Undo").keys("Ctrl-Z"), MenuItem::new("Redo").keys("Ctrl-Y")];
    let bar = menubar([menu_button("File", file), menu_button("Edit", edit), menu_button("View", edit)]);
    page(
        "Menu bar",
        "The application menus: a click opens a menu under its name.",
        [demo("Application", framed(bar), "menubar([menu_button(\"File\", || items), ..])")],
    )
}

pub fn statusbar_page() -> Div {
    let bar = statusbar()
        .left(status_label(Status::Running, "Connected"))
        .left(status_item(Some(icons::LAYERS), "cluster-prod"))
        .left(status_item(None, "default"))
        .right(status_item(None, "Ln 12, Col 4"))
        .right(status_item(None, "UTF-8"))
        .right(problems(icons::CIRCLE_X, token::HUE_RED, "2"))
        .right(problems(icons::TRIANGLE_ALERT, token::HUE_ORANGE, "5"))
        .right(status_item(Some(icons::BELL), "2"));
    page(
        "Status bar",
        "The bottom strip of a window: connection, context and quiet counters.",
        [demo(
            "Window footer",
            framed(column().child(div().height(60.0)).child(bar)),
            "statusbar().left(status_label(Status::Running, \"Connected\")).right(status_item(None, \"UTF-8\"))",
        )],
    )
}

/// A count of diagnostics: its icon in the hue of its kind, then the number.
fn problems(body: &'static str, hue: Token, count: &str) -> Div {
    let mark = icon(body).size(14.0).color(hue);
    row().align_items(quadra::Align::Center).gap(4.0).child(mark).child(quadra::text(count.to_string()))
}

pub fn palette_page(gallery: Gallery) -> Div {
    let open = button("Mở bảng lệnh").icon(icons::COMMAND).test_id("pal-open");
    let open = open.on_click(move || gallery.palette.set(true));
    page(
        "Command palette",
        "Fuzzy search over every command, with or without diacritics. Ctrl-K opens it anywhere.",
        [
            demo("Open", line().child(open).child(kbd("ctrl-k")), "command_palette(open, || commands)"),
            demo(
                "A picker under a control",
                cluster_picker(),
                "command_palette(open, clusters).under(&opener).width(280.0)",
            ),
        ],
    )
}

/// The clusters to switch to, hung under the name of the one in use.
fn cluster_picker() -> Div {
    let (open, at, using) = (signal(false), quadra::NodeRef::new(), signal("prod-cluster"));
    let clusters = move || {
        let all = [
            ("prod-cluster", Status::Running, "connected"),
            ("staging", Status::Running, "connected"),
            ("dev-eu-west-1", Status::Failed, "unreachable"),
        ];
        let command = |(name, status, words): (&'static str, Status, &str)| {
            let command = Command::new(name, move || using.set(name)).status(status, words);
            if name == using.get_untracked() { command.current() } else { command }
        };
        all.into_iter().map(command).collect()
    };
    let opener = button(move || using.get().to_string()).icon(icons::SERVER).on_click(move || open.set(true));
    let picker = command_palette(open, clusters).under(&at).width(280.0).placeholder("Switch cluster…");
    let picker = picker.foot("Enter switch · Esc close", |n| format!("{n} clusters"));
    line().child(div().node_ref(&at).child(opener)).child(picker)
}
