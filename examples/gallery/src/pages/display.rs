//! What shows and does nothing: status, badges, tags, keys, avatars, what is under way,
//! cards, alerts, empty places and lists of terms.

use quadra::{Align, AnyView, Div, column, div, row, text};
use quadra_design::{
    Status, Tone, alert, avatar, badge, button, card, card_head, copyable, description_list, empty_state,
    fonts, icons, kbd, meter, plain_badge, progress, progress_busy, size, skeleton, spinner, status_label,
    status_mark, tag, token,
};

use crate::common::{demo, framed, hint, line, mono, page, stack};

const STATES: [Status; 6] = [
    Status::Running,
    Status::Pending,
    Status::Failed,
    Status::Succeeded,
    Status::Terminating,
    Status::Unknown,
];

pub fn status_page() -> Div {
    let pods = [
        ("api-7d9f8c6b5-x2k4p", "Running", "0", "3d"),
        ("worker-5c6d7f8-9qwer", "ContainerCreating", "0", "14m"),
        ("migrate-28417-hk2zd", "CrashLoopBackOff", "5", "2m"),
        ("cron-backup-28417-1", "Completed", "0", "1h"),
        ("web-6b7c8d9e0-old01", "Terminating", "1", "6d"),
    ];
    let head = row().align_items(Align::Center).height(28.0).padding_x(12.0).bg(token::PANEL);
    let head = head.border_bottom(1.0).border_color(token::LINE).text_size(size::TEXT_SM.size);
    let head = head.text_color(token::TEXT_2).child(div().width(260.0).child(text("Name")));
    let head = head.child(div().width(200.0).child(text("Status")));
    let head = head.child(div().width(80.0).child(text("Restarts"))).child(text("Age"));
    let mut table = column().border(1.0).border_color(token::LINE).child(head);
    for (i, (name, state, restarts, age)) in pods.into_iter().enumerate() {
        let line = row().align_items(Align::Center).height(28.0).padding_x(12.0);
        let line = line.border_bottom(1.0).border_color(token::LINE).hover(|s| s.bg(token::PANEL_HOVER));
        let line = line.child(div().width(260.0).child(mono(name).text_color(token::TEXT)));
        let line = line.child(div().width(200.0).child(status_label(STATES[i], state)));
        table = table.child(line.child(div().width(80.0).child(mono(restarts))).child(mono(age)));
    }
    let about = "Workload state as a color and a shape, so it reads in grayscale and for color-blind users.";
    page(
        "Status",
        about,
        [
            demo(
                "States",
                line().children(STATES.map(|state| status_label(state, ""))),
                "status_label(Status::Failed, \"CrashLoopBackOff\")",
            ),
            demo("In a table", table, ""),
            demo("Marks only", line().children(STATES.map(status_mark)), "status_mark(Status::Running)"),
        ],
    )
}

pub fn badge_page() -> Div {
    let tones = [
        ("Neutral", Tone::Neutral),
        ("New", Tone::Primary),
        ("Healthy", Tone::Success),
        ("Degraded", Tone::Warning),
        ("Down", Tone::Danger),
        ("Beta", Tone::Info),
    ];
    // Kinds of objects, each in its hue and named.
    let kinds = [
        ("Pod", token::HUE_CYAN),
        ("Service", token::HUE_BLUE),
        ("Secret", token::HUE_PURPLE),
        ("Volume", token::HUE_ORANGE),
    ];
    let count = |name: &str, n: &str, tone| {
        row().align_items(Align::Center).gap(6.0).child(text(name.to_string())).child(badge(n, tone))
    };
    let counts = line()
        .child(count("Problems", "12", Tone::Danger))
        .child(count("Warnings", "3", Tone::Warning))
        .child(count("Changes", "148", Tone::Neutral));
    page(
        "Badge",
        "A short label for a count or a state, tinted by tone.",
        [
            demo(
                "Tones",
                line().children(tones.map(|(name, tone)| badge(name, tone))),
                "badge(\"Healthy\", Tone::Success)",
            ),
            demo("Counts", counts, ""),
            demo(
                "Kinds",
                line().children(kinds.map(|(name, hue)| badge(name, Tone::Hue(hue)))),
                "badge(\"Service\", Tone::Hue(token::HUE_BLUE))",
            ),
            demo(
                "Kinds as words, not code",
                line().children(kinds.map(|(name, hue)| plain_badge(name, Tone::Hue(hue)))),
                "plain_badge(\"Service\", Tone::Hue(token::HUE_BLUE))",
            ),
        ],
    )
}

pub fn tag_page() -> Div {
    let labels = ["app=api", "tier=backend", "env=prod", "v1.29.3"];
    page(
        "Tag",
        "Metadata such as labels, versions and selectors.",
        [demo("Labels", line().children(labels.map(tag)), "tag(\"app=api\")")],
    )
}

pub fn kbd_page() -> Div {
    let keys = [
        ("ctrl-k", "Command palette"),
        ("ctrl-p", "Go to file"),
        ("ctrl-shift-p", "Profiler"),
        ("ctrl-`", "Toggle terminal"),
        ("f2", "Rename"),
    ];
    let mut list = column().max_width(420.0);
    for (keys, does) in keys {
        let item = row().align_items(Align::Center).padding_y(6.0).border_bottom(1.0);
        list = list
            .child(item.border_color(token::LINE).child(div().grow(1.0).child(text(does))).child(kbd(keys)));
    }
    page(
        "Kbd",
        "A keyboard shortcut, written once and shown with the platform key names.",
        [demo("Shortcuts", list, "kbd(\"ctrl-shift-p\")")],
    )
}

pub fn avatar_page() -> Div {
    let people = [("Nguyễn Văn An", 40.0), ("Trần Bình", 32.0), ("Lê Thị Cúc", 24.0), ("Ada Lovelace", 20.0)];
    let author = row()
        .align_items(Align::Center)
        .gap(8.0)
        .child(avatar("Ada Lovelace", 20.0))
        .child(text("Ada Lovelace").weight(size::WEIGHT_STRONG))
        .child(hint("committed 2 hours ago"))
        .child(mono("a1b2c3d"));
    page(
        "Avatar",
        "A person, as initials on a color derived from the name.",
        [
            demo(
                "Sizes",
                line().children(people.map(|(name, side)| avatar(name, side))),
                "avatar(\"Nguyễn Văn An\", 32.0)",
            ),
            demo("Author line", author, ""),
        ],
    )
}

pub fn progress_page() -> Div {
    let bar = |part: f32, says: &str| {
        row()
            .align_items(Align::Center)
            .gap(12.0)
            .child(progress(part).width(320.0))
            .child(mono(says.to_string()))
    };
    page(
        "Progress bar",
        "Known or unknown progress. What moves is composed: nothing is laid out or painted.",
        [
            demo(
                "Determinate",
                stack().child(bar(0.25, "25%")).child(bar(0.7, "70%")),
                "progress(0.7).width(320.0)",
            ),
            demo("Indeterminate", progress_busy().width(320.0), "progress_busy()"),
            demo(
                "A meter, with a mark",
                stack()
                    .child(
                        row()
                            .align_items(Align::Center)
                            .gap(12.0)
                            .child(meter(0.43).mark(0.61).width(320.0))
                            .child(mono("CPU 43%, requested 61%".to_string())),
                    )
                    .child(
                        row()
                            .align_items(Align::Center)
                            .gap(12.0)
                            .child(meter(0.91).mark(0.7).warn_at(0.85).width(320.0))
                            .child(mono("memory 91%".to_string())),
                    ),
                "meter(used).mark(requested).warn_at(0.85)",
            ),
        ],
    )
}

pub fn spinner_page() -> Div {
    let sizes = line().children([12.0, 16.0, 24.0].map(|px| spinner().size(px)));
    let inline = row().align_items(Align::Center).gap(8.0);
    let inline = inline.child(spinner().size(12.0)).child(hint("Pulling image nginx:1.27…"));
    page(
        "Spinner",
        "Work in progress of unknown length, inline with the text it belongs to.",
        [demo("Sizes", sizes, "spinner().size(16.0)"), demo("Inline", inline, "")],
    )
}

pub fn skeleton_page() -> Div {
    let lines = stack().children([240.0, 320.0, 180.0].map(|width| skeleton(width, 12.0)));
    page(
        "Skeleton",
        "Placeholder shapes while content loads, in the layout of the real content.",
        [demo("Lines", lines, "skeleton(320.0, 12.0)")],
    )
}

pub fn card_page() -> Div {
    let metric = |title: &str, about: &str, value: &str| {
        let value = div().font_family(fonts::MONO).child(text(value.to_string()).size(22.0));
        card().child(card_head(title, about)).child(value)
    };
    let metrics = line()
        .child(metric("CPU", "cluster, 5 min", "42%"))
        .child(metric("Memory", "cluster, 5 min", "11.6 GiB"))
        .child(metric("Pods", "running / total", "118 / 124"));
    page(
        "Card",
        "Groups a few related values. Cards never nest; use a plane for regions.",
        [demo("Metrics", metrics, "card().child(card_head(\"CPU\", \"cluster, 5 min\")).child(body)")],
    )
}

pub fn alert_page() -> Div {
    let all = stack()
        .width(520.0)
        .child(alert(Tone::Info, "Update available", "Version 0.4 adds docking and Markdown help."))
        .child(alert(Tone::Success, "Deployed", "api is running 5 replicas."))
        .child(alert(Tone::Warning, "High memory", "node pool-a-3 is at 91%."))
        .child(alert(Tone::Danger, "Probe failed", "/healthz timed out 3 times."));
    page(
        "Alert",
        "A message in the page flow, tinted by its tone.",
        [demo(
            "Tones",
            row().child(all),
            "alert(Tone::Warning, \"High memory\", \"node pool-a-3 is at 91%.\")",
        )],
    )
}

pub fn empty_state_page() -> Div {
    let nothing = empty_state(
        icons::SEARCH,
        "No pods match",
        "Try another name or clear the filter.",
        button("Clear filter"),
    );
    let about = "What a list, table or search shows when it has nothing: say why and offer the next step.";
    page(
        "Empty state",
        about,
        [demo(
            "No results",
            row().child(framed(nothing).width(420.0)),
            "empty_state(icons::SEARCH, \"No pods match\", \"Try another name…\", action)",
        )],
    )
}

pub fn description_list_page() -> Div {
    let image = div().font_family(fonts::MONO).child(text("registry.example.com/api:1.8.2"));
    let items = [
        ("Name", AnyView::new(text("api-7d9f8c"))),
        ("Namespace", AnyView::new(text("production"))),
        ("Status", AnyView::new(badge("Running", Tone::Success))),
        ("Image", AnyView::new(image)),
    ];
    let copied = [
        ("Pod IP", copyable("10.42.0.17")),
        ("UID", copyable("8f2c41d0-5e1b-4a7e-9a3c-2b6f0e1d7c95")),
        ("Node", AnyView::new(text("worker-3"))),
    ];
    page(
        "Description list",
        "Terms and their values in two aligned columns: details panels and property sheets.",
        [
            demo(
                "Pod",
                description_list(items),
                "description_list([(\"Name\", AnyView::new(text(\"api\"))), ..])",
            ),
            demo("Values to copy", description_list(copied), "(\"Pod IP\", copyable(\"10.42.0.17\"))"),
        ],
    )
}
