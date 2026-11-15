//! What opens above the window: menus, tooltips, popovers, dialogs, questions and notices.

use quadra::{Align, AnyView, Div, Justify, Role, column, div, row, signal, text};
use quadra_design::{
    Edge, MenuItem, Notice, Size, Tone, Variant, badge, button, checkbox, confirm, context_menu,
    description_list, dialog, drawer, hover_card, icon_button, icons, menu_button, popover_button, size,
    token, tooltip,
};

use crate::common::{demo, hint, line, page};
use crate::shell::Gallery;

fn demo_menu(gallery: Gallery) -> Vec<MenuItem> {
    let copied = move || {
        gallery.notices.toast(Tone::Success, "Đã sao chép");
    };
    vec![
        MenuItem::new("Sao chép").icon(icons::COPY).keys("Ctrl-C").on_select(copied),
        MenuItem::new("Đổi tên").icon(icons::PENCIL).keys("F2"),
        MenuItem::new("Không khả dụng").disabled(true),
        MenuItem::separator(),
        MenuItem::new("Xoá").icon(icons::TRASH_2).danger(),
    ]
}

pub fn menu_page(gallery: Gallery) -> Div {
    let file = menu_button("Tệp", move || demo_menu(gallery)).icon(icons::FILE).test_id("mn-file");
    let more = menu_button("Actions", move || demo_menu(gallery)).variant(Variant::Outline);
    page(
        "Menu",
        "Actions in a popup, with icons and shortcuts. Arrow keys move, Enter picks, Escape closes.",
        [demo("Menu button", line().child(file).child(more), "menu_button(\"File\", || items)")],
    )
}

/// The menu of an editor: groups between lines, chords as shortcuts, and a row that does not
/// apply here.
fn editor_menu() -> Vec<MenuItem> {
    let rows = [
        ("Go to Definition", "f12"),
        ("Go to Implementation", "ctrl-f12"),
        ("Find All References", "alt-shift-f12"),
        ("-", ""),
        ("Rename Symbol", "f2"),
        ("Format Selections", "ctrl-k ctrl-f"),
        ("Show Code Actions", "ctrl-."),
        ("-", ""),
        ("Cut", "ctrl-x"),
        ("Copy", "ctrl-c"),
        ("Paste", "ctrl-v"),
        ("-", ""),
        ("Open in Terminal", ""),
        ("Copy Permalink to Line", "x"),
    ];
    let item = |(name, keys): (&str, &str)| match (name, keys) {
        ("-", _) => MenuItem::separator(),
        (name, "x") => MenuItem::new(name).disabled(true),
        (name, "") => MenuItem::new(name),
        (name, keys) => MenuItem::new(name).keys(quadra_design::key_label(keys)),
    };
    rows.into_iter().map(item).collect()
}

pub fn context_menu_page() -> Div {
    let target = row().align_items(Align::Center).justify(Justify::Center).size(320.0, 96.0);
    let target = target.border(1.0).border_color(token::LINE_STRONG).child(hint("Right-click here"));
    page(
        "Context menu",
        "The same menu, opened by a right click on the target.",
        [demo("Target", row().child(context_menu(target, editor_menu)), "context_menu(target, || items)")],
    )
}

pub fn tooltip_page() -> Div {
    let hover = line()
        .child(tooltip("Tooltips appear after 500 ms", button("Hover me").test_id("tt-btn")))
        .child(tooltip("Refresh", icon_button(icons::REFRESH_CW, "Refresh")).keys("F5"));
    let detail = tooltip("Deploy to staging", button("Deploy")).keys("Ctrl-Shift-D");
    page(
        "Tooltip",
        "A short label after a 500 ms hover. Never put actions inside.",
        [
            demo("Hover", hover, "tooltip(\"Refresh\", view).keys(\"F5\")"),
            demo(
                "With detail",
                line().child(detail.meta("Builds the current branch first")),
                "tooltip(\"Deploy to staging\", view).keys(\"Ctrl-Shift-D\").meta(\"..\")",
            ),
        ],
    )
}

pub fn popover_page() -> Div {
    let columns = popover_button("Columns", || {
        let title = text("Visible columns").weight(size::WEIGHT_STRONG);
        column()
            .gap(8.0)
            .width(220.0)
            .child(title)
            .child(checkbox(signal(true)).label("Name"))
            .child(checkbox(signal(true)).label("Age"))
            .child(checkbox(signal(false)).label("Node"))
    });
    let about = "A framed panel next to its trigger for settings or details: a click opens it, a click \
                 outside or Escape closes it.";
    page(
        "Popover",
        about,
        [demo(
            "Filter",
            line().child(columns.icon(icons::SLIDERS_HORIZONTAL)),
            "popover_button(\"Columns\", || content)",
        )],
    )
}

pub fn hover_card_page() -> Div {
    let name = "api-7d9f8c6b5-x2k9q";
    let pod = div().role(Role::Link).test_id("hc-trigger").child(text(name).color(token::ACCENT));
    let card = hover_card(pod, move || {
        let logs = button("Open logs").size(Size::Sm);
        let title = text(name).weight(size::WEIGHT_STRONG);
        column().gap(4.0).width(240.0).child(title).child(text("Running on node-2 · 3 restarts")).child(logs)
    });
    let about = "Details about a link or a name after a 400 ms hover. It stays open while the pointer is \
                 over it, so its links can be clicked.";
    page("Hover card", about, [demo("Pod", card, "hover_card(trigger, || content)")])
}

pub fn drawer_page() -> Div {
    let body = || {
        description_list([
            ("Status", AnyView::new(badge("Running", Tone::Success))),
            ("Node", AnyView::new(text("pool-a-3"))),
            ("Restarts", AnyView::new(text("2"))),
        ])
    };
    let mut sides = line();
    for (edge, name, reach) in
        [(Edge::Right, "Right", 360.0), (Edge::Left, "Left", 360.0), (Edge::Bottom, "Bottom", 240.0)]
    {
        let open = signal(false);
        let variant = if edge == Edge::Right { Variant::Primary } else { Variant::Secondary };
        let opener = button(name).variant(variant).on_click(move || open.set(true));
        let opener = if edge == Edge::Right { opener.test_id("drawer-right") } else { opener };
        sides = sides.child(opener).child(drawer(open, edge, "Pod api-7d9f8c", body).size(reach));
    }
    let about = "A panel over the window along an edge, for details and settings. Escape or a click \
                 outside closes it.";
    page(
        "Drawer",
        about,
        [demo("Sides", sides, "drawer(open, Edge::Right, \"Pod api-7d9f8c\", || body).size(360.0)")],
    )
}

pub fn dialog_page() -> Div {
    let open = signal(false);
    let ask = button("Scale deployment…").variant(Variant::Primary).test_id("dl-open");
    let scale = dialog(open, "Scale deployment", || text("api will be scaled from 3 to 5 replicas."));
    let about = "A modal task hanging near the top of the window. Escape, a click outside and the close \
                 button dismiss it.";
    page(
        "Dialog",
        about,
        [demo(
            "Modal",
            column().child(ask.on_click(move || open.set(true))).child(scale),
            "dialog(open, \"Scale deployment\", || body)",
        )],
    )
}

pub fn confirm_page() -> Div {
    let (open, answer) = (signal(false), signal("—"));
    let ask =
        button("Delete pod").variant(Variant::Danger).test_id("cf-open").on_click(move || open.set(true));
    let sure = confirm(open, "Delete pod?", "api-7d9f8c6b5-x2k4p will be terminated.", "Delete", move || {
        answer.set("deleted")
    });
    let said = hint(move || format!("Answer: {}", answer.get()));
    page(
        "Confirm",
        "A yes-or-no question before a destructive action.",
        [demo(
            "Delete",
            line().child(ask).child(said).child(sure),
            "confirm(open, \"Delete pod?\", message, \"Delete\", || ..)",
        )],
    )
}

pub fn toast_page(gallery: Gallery) -> Div {
    let all = gallery.notices;
    let says = move |tone, words: &'static str| {
        move || {
            all.toast(tone, words);
        }
    };
    let kinds = line()
        .child(button("Info").test_id("to-info").on_click(says(Tone::Info, "Port-forward started on :8080")))
        .child(button("Success").on_click(says(Tone::Success, "Deployment rolled out")))
        .child(button("Warning").on_click(says(Tone::Warning, "Node pool-a-3 is under memory pressure")))
        .child(
            button("Error").test_id("to-err").on_click(says(Tone::Danger, "Image pull failed: nginx:1.99")),
        );
    let notify = move || {
        let ready = "v-analyzer 0.0.6 is ready. It starts with the next file you open.";
        let installed = Notice::new("Language server installed").body(ready);
        all.notify(installed.action("Release Notes", || {}).primary("Open a File", || {}));
        let saved = Notice::new("Saved").body("This one closes by itself after 5 s.").tone(Tone::Success);
        all.notify(saved.lasts(std::time::Duration::from_secs(5)));
    };
    let about = "A brief notice at the bottom center that expires after 10 s; notifications stay at the \
                 bottom right until closed.";
    page(
        "Toast",
        about,
        [
            demo("Kinds", kinds, "notices.toast(Tone::Success, \"Deployment rolled out\")"),
            demo(
                "Notification",
                line().child(button("Notify").on_click(notify)),
                "notices.notify(Notice::new(\"Language server installed\").body(\"..\"))",
            ),
        ],
    )
}
