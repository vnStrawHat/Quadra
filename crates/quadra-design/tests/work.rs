//! The workbench (specs/behaviour/16): the activity bar, the panel header and the tab strip.

use std::rc::Rc;
use std::time::Duration;

use quadra_design::work::{Activity, Tab, activity_bar, panel_header, tab_strip};
use quadra_design::{Size, dark, icon_button, icons};
use quadra_platform::{Mods, MouseButton};
use quadra_prim::Point;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role, SemanticAction};
use quadra_view::{View, column, row};

fn mounted<V: View>(build: impl FnOnce() -> V) -> Harness {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(build);
    h.frame();
    h
}

fn wait(h: &mut Harness, ms: u64) {
    h.advance(Duration::from_millis(ms));
    h.frame();
    while h.frame_if_needed().is_some() {}
}

#[test]
fn an_activity_bar_shows_the_tools_and_a_click_chooses_one() {
    let (active, waiting) = (signal("files"), signal(3u32));
    let mut h = mounted(move || {
        let tools = [
            Activity::new("files", icons::FILE, "Files"),
            Activity::new("events", icons::BELL, "Events").badge(move || waiting.get()),
        ];
        let bar =
            activity_bar(active, tools).footer([Activity::new("settings", icons::SETTINGS, "Settings")]);
        quadra_design::root().child(row().height(300.0).child(bar))
    });
    let tree = h.semantics();
    for line in ["TabList", "Tab \"Files\" [selected]", "Tab \"Events\"", "Tab \"Settings\""] {
        assert!(tree.contains(line), "{line} is not in:\n{tree}");
    }
    // 40 px wide with its line, a tool 40 px high, the footer at the bottom.
    let (files, settings) =
        (h.get_by_role(Role::Tab, "Files").node(), h.get_by_role(Role::Tab, "Settings").node());
    let (files, settings) = (h.tree.window_rect(files), h.tree.window_rect(settings));
    assert_eq!((files.x, files.width, files.height, settings.bottom()), (0.0, 39.0, 40.0, 300.0));

    // A count shows while there is something to count, and "99+" over ninety-nine.
    assert!(h.has_text("3"));
    waiting.set(0);
    wait(&mut h, 0);
    assert!(!h.has_text("3") && !h.has_text("0"));
    waiting.set(120);
    wait(&mut h, 0);
    assert!(h.has_text("99+"));

    // A click chooses a tool; its name shows at its right once the pointer has rested.
    let events = h.get_by_role(Role::Tab, "Events").node();
    let events = h.tree.window_rect(events);
    h.click(events.center().x, events.center().y);
    wait(&mut h, 0);
    assert_eq!(active.get_untracked(), "events");
    assert!(h.semantics().contains("Tab \"Events\" [selected]"), "{}", h.semantics());
    h.pointer(PointerInput::Move, 600.0, 200.0);
    h.pointer(PointerInput::Move, events.center().x, events.center().y);
    wait(&mut h, 600);
    let tip = h.tree.window_rect(h.tree.overlays().next().unwrap());
    assert!(h.semantics().contains("Tooltip") && tip.x == events.right() + 6.0, "{tip:?}");
}

#[test]
fn a_panel_header_is_a_strip_with_its_title_and_what_is_added_at_its_end() {
    let mut h = mounted(|| {
        let head = panel_header("Pods").child(icon_button(icons::FILTER, "Filter").size(Size::Xs));
        quadra_design::root().child(column().width(320.0).child(head))
    });
    let (title, filter) = (h.get_by_text("Pods").node(), h.get_by_role(Role::Button, "Filter").node());
    let (title, filter) = (h.tree.window_rect(title), h.tree.window_rect(filter));
    // 28 px with its line: the title 8 px in, the button 4 px from the end, both in the middle.
    assert_eq!((title.x, filter.right(), filter.center().y), (8.0, 316.0, 13.5));
}

fn ids(tabs: quadra_reactive::Signal<Vec<Tab>>) -> String {
    tabs.with_untracked(|all| all.iter().map(|tab| tab.id.as_str()).collect::<Vec<_>>().join(" "))
}

#[test]
fn a_tab_is_chosen_by_a_press_moved_by_a_drag_and_closed() {
    let tabs =
        signal(vec![Tab::new("a", "Alpha"), Tab::new("b", "Beta").dirty(true), Tab::new("c", "Gamma")]);
    let active = signal("a".to_string());
    let mut h = mounted(move || {
        let strip =
            tab_strip(tabs, active).on_close(move |id| tabs.update(|all| all.retain(|tab| tab.id != id)));
        quadra_design::root().child(column().width(600.0).child(strip))
    });
    let tree = h.semantics();
    for line in ["TabList", "Tab \"Alpha\" [selected]", "Tab \"Beta\"", "Tab \"Gamma\""] {
        assert!(tree.contains(line), "{line} is not in:\n{tree}");
    }
    let rect = |h: &mut Harness, name: &str| {
        let tab = h.get_by_role(Role::Tab, name).node();
        h.tree.window_rect(tab)
    };
    let (alpha, beta, gamma) = (rect(&mut h, "Alpha"), rect(&mut h, "Beta"), rect(&mut h, "Gamma"));
    // 32 px high, one after another from the start of the strip.
    assert_eq!((alpha.x, alpha.height, beta.x, gamma.x), (0.0, 32.0, alpha.right(), beta.right()));

    // A press chooses a tab.
    h.pointer(PointerInput::Down, beta.center().x, 16.0);
    h.pointer(PointerInput::Up, beta.center().x, 16.0);
    wait(&mut h, 0);
    assert_eq!(active.get_untracked(), "b");
    assert!(h.semantics().contains("Tab \"Beta\" [selected]"), "{}", h.semantics());

    // Held and moved less than 4 px, a tab stays; moved over another, it takes its place.
    h.pointer(PointerInput::Down, alpha.center().x, 16.0);
    h.pointer(PointerInput::Move, alpha.center().x + 3.0, 16.0);
    assert_eq!(ids(tabs), "a b c");
    h.pointer(PointerInput::Move, gamma.center().x, 16.0);
    wait(&mut h, 0);
    h.pointer(PointerInput::Up, gamma.center().x, 16.0);
    assert_eq!((ids(tabs), active.get_untracked()), ("b c a".to_string(), "a".to_string()));
    assert_eq!(rect(&mut h, "Beta").x, 0.0);

    // The middle button closes a tab, and so does its cross.
    let gamma = rect(&mut h, "Gamma");
    let at = Point::new(gamma.center().x, 16.0);
    h.tree.pointer(PointerInput::Down, MouseButton::Middle, at, Mods::NONE, h.now());
    h.tree.pointer(PointerInput::Up, MouseButton::Middle, at, Mods::NONE, h.now());
    wait(&mut h, 0);
    assert_eq!(ids(tabs), "b a");
    let crosses = h.query(|n| n.semantics.role == Role::Button && n.semantics.name == "Close");
    assert!(h.tree.act(crosses[0], &SemanticAction::Click));
    wait(&mut h, 0);
    assert_eq!((ids(tabs), active.get_untracked()), ("a".to_string(), "a".to_string()));
}

#[test]
fn the_list_at_the_end_of_a_strip_names_every_tab() {
    let tabs = signal(vec![Tab::new("a", "Alpha"), Tab::new("b", "Beta")]);
    let active = signal("a".to_string());
    let mut h =
        mounted(move || quadra_design::root().child(column().width(600.0).child(tab_strip(tabs, active))));
    // A strip that was not told how to close has no crosses.
    assert!(h.query(|n| n.semantics.name == "Close").is_empty());
    h.get_by_role(Role::Button, "Tabs").click();
    wait(&mut h, 0);
    let tree = h.semantics();
    assert!(tree.contains("MenuItem \"Alpha\" [checked]") && tree.contains("MenuItem \"Beta\""), "{tree}");
    h.get_by_role(Role::MenuItem, "Beta").click();
    wait(&mut h, 0);
    assert_eq!((active.get_untracked(), h.tree.overlays().count()), ("b".to_string(), 0));
}

#[test]
fn a_tab_is_renamed_in_place_has_its_own_menu_and_a_long_title_is_cut() {
    use quadra_design::MenuItem;
    use quadra_platform::Key;
    let long = "A title far longer than any tab may be wide, which is cut";
    let tabs = signal(vec![Tab::new("a", "Alpha"), Tab::new("b", long)]);
    let (active, asked) = (signal("a".to_string()), signal(String::new()));
    let mut h = mounted(move || {
        let strip = tab_strip(tabs, active).on_rename(move |id, title| {
            tabs.update(|all| {
                all.iter_mut().filter(|tab| tab.id == id).for_each(|tab| tab.title = title.into())
            })
        });
        let strip = strip.menu(move |id| {
            let id = id.to_string();
            vec![MenuItem::new("Close others").on_select(move || asked.set(id.clone()))]
        });
        quadra_design::root().child(column().width(800.0).child(strip))
    });
    // A long title is cut at 200 px.
    let words =
        |h: &Harness, said: &str| h.query(|n| n.semantics.role == Role::Text && n.semantics.name == said)[0];
    let title = words(&h, long);
    assert_eq!(h.tree.window_rect(title).width, 200.0);

    // A double click puts the title in a field; Enter renames.
    let node = words(&h, "Alpha");
    let at = h.tree.window_rect(node).center();
    for _ in 0..2 {
        h.pointer(PointerInput::Down, at.x, at.y);
        h.pointer(PointerInput::Up, at.x, at.y);
    }
    wait(&mut h, 0);
    let field = h.get_by_role(Role::TextInput, "").node();
    assert_eq!(h.tree.focused(), Some(field));
    h.key(Key::End, Mods::NONE);
    h.type_text(" two");
    h.key(Key::Enter, Mods::NONE);
    wait(&mut h, 0);
    assert!(h.has_text("Alpha two") && h.query(|n| n.semantics.role == Role::TextInput).is_empty());
    // Escape gives it up.
    let node = words(&h, "Alpha two");
    let at = h.tree.window_rect(node).center();
    for _ in 0..2 {
        h.pointer(PointerInput::Down, at.x, at.y);
        h.pointer(PointerInput::Up, at.x, at.y);
    }
    wait(&mut h, 0);
    h.type_text("x");
    h.key(Key::Escape, Mods::NONE);
    wait(&mut h, 0);
    assert!(h.has_text("Alpha two") && !h.has_text("Alpha twox"));

    // The other button on a tab opens its own menu.
    let node = words(&h, long);
    let at = h.tree.window_rect(node).center();
    h.tree.pointer(PointerInput::Down, MouseButton::Right, at, Mods::NONE, h.now());
    h.tree.pointer(PointerInput::Up, MouseButton::Right, at, Mods::NONE, h.now());
    wait(&mut h, 0);
    h.get_by_role(Role::MenuItem, "Close others").click();
    assert_eq!(asked.get_untracked(), "b");
}

#[test]
fn the_tab_that_becomes_active_is_scrolled_to_in_a_strip_too_short_for_all() {
    let tabs = signal((0..12).map(|i| Tab::new(format!("t{i}"), format!("Tab {i}"))).collect::<Vec<_>>());
    let active = signal("t0".to_string());
    let mut h =
        mounted(move || quadra_design::root().child(column().width(300.0).child(tab_strip(tabs, active))));
    let shows = |h: &mut Harness, name: &str| {
        let tab = h.get_by_role(Role::Tab, name).node();
        let r = h.tree.window_rect(tab);
        r.x >= -0.01 && r.right() <= 300.01
    };
    assert!(!shows(&mut h, "Tab 11"));
    // Chosen from the menu of tabs, or opened past the end: it shows.
    active.set("t11".into());
    wait(&mut h, 0);
    assert!(shows(&mut h, "Tab 11"));
    tabs.update(|all| all.push(Tab::new("new", "New")));
    active.set("new".into());
    wait(&mut h, 0);
    assert!(shows(&mut h, "New"));
}
