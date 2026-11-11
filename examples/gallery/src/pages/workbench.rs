//! What a workbench is made of: the activity bar, the tab strip and the panel header.

use quadra::{Div, column, div, row, signal};
use quadra_kit::work::{Activity, Tab, activity_bar, panel_header, tab_strip};
use quadra_kit::{Size, icon_button, icons, token, tooltip};

use crate::common::{demo, framed, hint, mono, page};

pub fn activity_page() -> Div {
    let active = signal("workloads");
    let tools = [
        Activity::new("workloads", icons::LAYERS, "Workloads"),
        Activity::new("config", icons::FILE, "Config"),
        Activity::new("search", icons::SEARCH, "Search"),
        Activity::new("events", icons::BELL, "Events").badge(|| 12),
    ];
    let bar = activity_bar(active, tools).footer([Activity::new("settings", icons::SETTINGS, "Settings")]);
    let said = div().grow(1.0).padding(16.0).child(hint(move || format!("Active: {}", active.get())));
    let about = "The leftmost column: one icon per tool view, the active one in the accent, badges for \
                 counts.";
    page(
        "Activity bar",
        about,
        [demo(
            "Cluster tools",
            framed(row().height(280.0).child(bar).child(said)),
            "activity_bar(active, [Activity::new(\"events\", icons::BELL, \"Events\").badge(|| 12), ..])",
        )],
    )
}

pub fn tab_strip_page() -> Div {
    let tabs = signal(vec![
        Tab::new("deploy", "api.yaml").icon(icons::FILE),
        Tab::new("logs", "Logs: api-7d9f8c").icon(icons::LIST),
        Tab::new("shell", "Shell").icon(icons::MONITOR),
        Tab::new("main", "main.rs").icon(icons::FILE).dirty(true),
    ]);
    let active = signal("logs".to_string());
    let strip = tab_strip(tabs, active).on_close(move |id| tabs.update(|all| all.retain(|tab| tab.id != id)));
    let shown = div().grow(1.0).padding(16.0).bg(token::CANVAS).child(mono(move || active.get()));
    let about = "Open editors and views of one pane. Drag to reorder, middle-click or × to close; a dot \
                 marks unsaved changes.";
    page(
        "Tab strip",
        about,
        [demo(
            "Editor tabs",
            framed(column().height(160.0).child(strip).child(shown)),
            "tab_strip(tabs, active).on_close(|id| ..)",
        )],
    )
}

pub fn panel_header_page() -> Div {
    let quiet = |body, name: &'static str| tooltip(name, icon_button(body, name).size(Size::Xs));
    let head = panel_header("Pods")
        .child(quiet(icons::FILTER, "Filter"))
        .child(quiet(icons::REFRESH_CW, "Refresh"))
        .child(quiet(icons::ELLIPSIS, "More"));
    let panel =
        column().width(320.0).child(head).child(div().height(80.0).padding(12.0).child(hint("124 pods")));
    page(
        "Panel header",
        "The title strip of a panel: a section label and quiet actions.",
        [demo("Explorer", framed(panel), "panel_header(\"Pods\").child(icon_button(..))")],
    )
}
