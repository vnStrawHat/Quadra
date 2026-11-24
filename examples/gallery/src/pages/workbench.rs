//! What a workbench is made of: the activity bar, the tab strip, the panel header, the pane
//! tree, docking and the settings.

use quadra::{Div, column, div, dynamic, memo, row, signal};
use quadra_design::work::{Activity, Axis, Pane, Tab, activity_bar, dock, panel_header, panes, tab_strip};
use quadra_design::{
    SettingsGroup, SettingsPage, Size, Tone, Variant, button, dialog, icon_button, icons, number_input,
    segmented, select, settings, settings_group, switch, token, tooltip,
};

use crate::common::{demo, framed, hint, line, mono, page, stack};

pub fn activity_page() -> Div {
    let active = signal("workloads");
    let tools = [
        Activity::new("workloads", icons::LAYERS, "Workloads").badge(|| 3).badge_tone(Tone::Danger),
        Activity::new("config", icons::FILE, "Config"),
        Activity::new("search", icons::SEARCH, "Search"),
        Activity::new("events", icons::BELL, "Events").badge(|| 12),
    ];
    let bar = activity_bar(active, tools).footer([Activity::new("settings", icons::SETTINGS, "Settings")]);
    let said =
        div().grow(1.0).padding(token::SPACE_XL).child(hint(move || format!("Active: {}", active.get())));
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
    let shown = div().grow(1.0).padding(token::SPACE_XL).bg(token::CANVAS).child(mono(move || active.get()));
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
    let panel = column()
        .width(320.0)
        .child(head)
        .child(div().height(80.0).padding(token::SPACE_LG).child(hint("124 pods")));
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
        let said = div().padding(token::SPACE_LG).child(hint(format!("pane \"{group}\"")));
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
        let shown = div().padding(token::SPACE_LG).child(mono(move || tabs.active.get()));
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

pub fn settings_page() -> Div {
    let (current, query, scope, open) = (signal(0), signal(String::new()), signal(0), signal(false));
    let (accessible, trust, blink) = (signal(false), signal(true), signal(true));
    let (closing, quit, mode) = (signal(0), signal(0), signal(2));
    let general = SettingsGroup::new("General Settings")
        .row(
            "Accessible Mode",
            "Keep collapsed controls expanded and reachable from the keyboard, for screen readers.",
            switch(accessible),
        )
        .resets(accessible, false)
        .row(
            "When Closing With No Tabs",
            "What closing the active item does when no tab is open.",
            select(["Platform Default", "Close Window", "Keep Window Open"], closing).width(220.0),
        )
        .resets(closing, 0)
        .row(
            "On Last Window Closed",
            "What happens when the last window is closed.",
            select(["Platform Default", "Quit App"], quit).width(220.0),
        )
        .resets(quit, 0);
    let security = SettingsGroup::new("Security")
        .row(
            "Trust All Projects",
            "Run project tasks and language servers without asking first.",
            switch(trust),
        )
        .resets(trust, true);
    let theme = SettingsGroup::new("Theme")
        .row(
            "Theme Mode",
            "Follow the system, or always use the light or the dark theme.",
            segmented(["Light", "Dark", "System"], mode),
        )
        .resets(mode, 2);
    let cursor = SettingsGroup::new("Cursor")
        .row("Cursor Blink", "Blink the text cursor while the editor has focus.", switch(blink))
        .resets(blink, true);
    let json = move || {
        let mode = ["light", "dark", "system"][mode.get()];
        let (accessible, trust, blink) = (accessible.get(), trust.get(), blink.get());
        format!(
            "{{\n  \"accessible_mode\": {accessible},\n  \"trust_all_projects\": {trust},\n  \
             \"cursor_blink\": {blink},\n  \"theme_mode\": \"{mode}\"\n}}"
        )
    };
    let small = |name: &str| button(name.to_string()).size(Size::Sm);
    let mut view = settings(current)
        .page(SettingsPage::new("General").group(general).group(security))
        .page(SettingsPage::new("Appearance").group(theme))
        .page(SettingsPage::new("Editor").group(cursor))
        .search(query)
        .hint("ctrl-shift-e", "Focus Navbar");
    for name in ["Keymap", "Languages & Tools", "Window & Layout", "Terminal"] {
        view = view.page(SettingsPage::new(name));
    }
    for (i, name) in ["User", "Project"].into_iter().enumerate() {
        let scope_is = small(name).variant(Variant::Ghost).toggled(move || scope.get() == i);
        view = view.header(scope_is.on_click(move || scope.set(i)));
    }
    let edit = small("Edit in settings.json").variant(Variant::Outline).on_click(move || open.set(true));
    let view = view.header(div().grow(1.0)).header(edit);
    let window = column().height(560.0).border(1.0).border_color(token::LINE_STRONG);
    let file = dialog(open, "settings.json", move || mono(json()));
    let about =
        "Preferences in pages: a searchable navigation, then rows of a title, a description and a control.";
    page(
        "Settings",
        about,
        [
            demo(
                "Window",
                window.child(view).child(file),
                "settings(current).page(SettingsPage::new(\"General\").group(..)).search(query)",
            ),
            demo("Groups in a page", groups(), "settings_group(SettingsGroup::new(\"Theme\").row(..))"),
        ],
    )
}

/// Groups of settings on their own, in a page of the app's: titled outlined cards.
fn groups() -> Div {
    let (mode, theme, font, fps, vertical) =
        (signal(1usize), signal(0usize), signal(16.0f64), signal(false), signal(false));
    let theme_group = SettingsGroup::new("Theme")
        .row(
            "Mode",
            "Switching this also swaps the colour theme to the last one used in that mode.",
            select(["Light", "Dark", "System"], mode).width(120.0),
        )
        .row(
            "Color theme",
            "Built-in and loaded themes.",
            select(["One Dark", "One Light"], theme).width(160.0),
        );
    let interface = SettingsGroup::new("Interface")
        .row(
            "UI font size",
            "Size of all text of the interface, in px.",
            number_input(font).range(10.0, 24.0).stepper(),
        )
        .resets(font, 16.0)
        .row("Show FPS monitor", "Frame times and dropped frames in the corner.", switch(fps))
        .resets(fps, false)
        .row("Vertical tabs", "Tabs in a list at the left instead of a strip above.", switch(vertical))
        .resets(vertical, false);
    column()
        .gap(token::SPACE_XXL)
        .width(640.0)
        .child(settings_group(theme_group))
        .child(settings_group(interface))
}
