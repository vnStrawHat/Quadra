//! What a workbench is made of: the activity bar, the tab strip, the panel header, the pane
//! tree and docking.

use quadra::{Div, column, div, dynamic, memo, row, signal};
use quadra_kit::work::{Activity, Axis, Pane, Tab, activity_bar, dock, panel_header, panes, tab_strip};
use quadra_kit::{Size, button, icon_button, icons, token, tooltip};

use crate::common::{demo, framed, hint, line, mono, page, stack};

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

pub fn panes_page() -> Div {
    let editor = Pane::column(Pane::leaf("editor"), Pane::leaf("terminal"), 0.65);
    let root = signal(Pane::row(Pane::leaf("explorer"), editor, 0.28));
    let folded = memo(move || root.with(|tree| tree.is_collapsed("terminal")));
    let leaf = move |group: &str| {
        let fold = move || {
            let (body, name) =
                if folded.get() { (icons::CHEVRON_UP, "Expand") } else { (icons::CHEVRON_DOWN, "Collapse") };
            let fold = icon_button(body, name).size(Size::Xs);
            tooltip(name, fold.on_click(move || root.update(|tree| tree.toggle("terminal"))))
        };
        let head = panel_header(group);
        let head = if group == "terminal" { head.child(dynamic(fold)) } else { head };
        let said = div().padding(12.0).child(hint(format!("pane \"{group}\"")));
        let plane = if group == "editor" { token::CANVAS } else { token::PANEL };
        column().grow(1.0).bg(plane).child(head).child(said)
    };
    let split = move || {
        root.update(|tree| {
            let new = format!("editor {}", tree.groups().len());
            tree.split("editor", Axis::Row, &new, false)
        })
    };
    let split = button("Split editor").icon(icons::PANEL_LEFT).size(Size::Sm).on_click(split);
    let count = hint(move || format!("{} panes", root.with(|tree| tree.groups().len())));
    let tree = framed(panes(root, leaf).header(28.0)).height(320.0);
    let about = "Nested splits whose leaves are tab groups. The tree is plain data: keep it in your model \
                 and save it with your settings.";
    page(
        "Pane tree",
        about,
        [demo(
            "Workbench",
            stack().child(line().child(split).child(count)).child(tree),
            "panes(root, |group| view).header(28.0)  // root.update(|tree| tree.toggle(\"terminal\"))",
        )],
    )
}

pub fn dock_page() -> Div {
    let file = |id: &str, title: &str| Tab::new(id, title).icon(icons::FILE);
    let left = vec![file("main", "main.rs"), file("app", "app.rs")];
    let right =
        vec![Tab::new("logs", "Logs").icon(icons::LIST), Tab::new("shell", "Shell").icon(icons::MONITOR)];
    let work =
        dock(Pane::row(Pane::leaf("left"), Pane::leaf("right"), 0.5), [("left", left), ("right", right)]);
    let leaf = move |group: &str| {
        let pane = column().grow(1.0).bg(token::CANVAS);
        let Some(tabs) = work.group(group) else { return pane };
        let shown = div().padding(12.0).child(mono(move || tabs.active.get()));
        pane.child(tab_strip(tabs.tabs, tabs.active).dock(work, group)).child(shown)
    };
    let about = "Tear a tab off its strip (drag it down) and drop it on another strip, the middle of a \
                 pane, or an edge to split there.";
    page(
        "Dock",
        about,
        [demo(
            "Docking workbench",
            framed(panes(work.root, leaf).dock(work)).height(320.0),
            "panes(work.root, |group| ..).dock(work)  // strips: tab_strip(tabs, active).dock(work, group)",
        )],
    )
}
