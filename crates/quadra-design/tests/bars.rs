//! The bars and the sidebar of a window (specs/behaviour/15, "Chrome"): the toolbar, the
//! status bar and what stands in it, the breadcrumb, the marks of a status, and the places
//! of an app in a sidebar that folds.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_design::{
    NavItem, Status, breadcrumb, button, dark, icons, sidebar, status_button, status_divider, status_item,
    status_label, status_mark, statusbar, token, toolbar,
};
use quadra_platform::{Key, Mods};
use quadra_prim::{Color, Size};
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_view::{View, column, div, row};

fn mount<V: View + 'static>(build: impl FnOnce() -> V + 'static) -> Harness {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || quadra_design::root().child(build()));
    h.frame();
    h
}

fn settle(h: &mut Harness) {
    h.frame();
    while h.frame_if_needed().is_some() {}
}

#[test]
fn the_bars_have_their_heights_and_the_breadcrumb_its_links() {
    let (panel, picked) = (signal(true), Rc::new(RefCell::new(Vec::new())));
    let log = picked.clone();
    let mut h = mount(move || {
        let way = breadcrumb(["home", "notes", "today.md"], move |i| log.borrow_mut().push(i));
        let tools = toolbar().child(button("Run").test_id("run")).child(way);
        let toggle = status_button(icons::TERMINAL, "Terminal", move || panel.get());
        let toggle = toggle.on_click(move || panel.update(|on| *on = !*on));
        let status = statusbar()
            .left(status_item(Some(icons::CHECK), "Saved"))
            .left(status_divider())
            .right(status_item(None, "UTF-8"))
            .right(toggle);
        column().grow(1.0).child(tools).child(div().grow(1.0)).child(status)
    });
    let tree = h.semantics();
    for line in [
        "Link \"home\"",
        "Link \"notes\"",
        "Text \"today.md\"",
        "Text \"Saved\"",
        "Button \"Terminal\" [checked]",
    ] {
        assert!(tree.contains(line), "{line} is not in:\n{tree}");
    }
    // The toolbar is 45 px under the top, the status bar 30 at the bottom of the window.
    let (run, bar) = (h.get_by_test_id("run").node(), h.get_by_test_id("statusbar").node());
    let (tools, bar) = (h.tree.window_rect(h.tree.parent(run).unwrap()), h.tree.window_rect(bar));
    assert_eq!((tools.height, bar.y, bar.height), (45.0, 570.0, 30.0));
    // What is at the right is at the right: the toggle ends 8 px from the edge.
    let toggle = h.get_by_role(Role::Button, "Terminal").node();
    assert_eq!(
        (h.tree.window_rect(toggle).right(), h.tree.rect(toggle).size()),
        (792.0, Size::new(22.0, 22.0))
    );
    // On, an icon alone has the accent and no fill; off, the second text colour.
    let theme = dark().theme();
    let look = |h: &mut Harness| {
        let style = h.tree.paint_style(toggle);
        (style.background, h.tree.text_values(toggle).color)
    };
    assert_eq!(look(&mut h), (Color::TRANSPARENT, Some(theme.resolve(token::ACCENT_TEXT.into()))));
    h.get_by_role(Role::Button, "Terminal").click();
    settle(&mut h);
    assert_eq!(look(&mut h), (Color::TRANSPARENT, Some(theme.resolve(token::TEXT_2.into()))));

    // A name on the way is a link, muted; the last one is where the user is, in the text's
    // colour. 8 px between a name and a mark.
    let color = |h: &mut Harness, name: &str| {
        let node = h.get_by_text(name).node();
        h.tree.inherited(node).color
    };
    let (muted, plain) = (theme.resolve(token::TEXT_2.into()), theme.resolve(token::TEXT.into()));
    assert_eq!((color(&mut h, "notes"), color(&mut h, "today.md")), (muted, plain));
    let notes = h.get_by_text("notes").node();
    let mark = h.query(|n| n.semantics.name == "\u{203A}")[1];
    let (notes, mark) = (h.tree.window_rect(notes), h.tree.window_rect(mark));
    assert!((mark.x - notes.right() - 8.0).abs() < 0.01, "{notes:?} {mark:?}");
    h.get_by_role(Role::Link, "notes").click();
    h.get_by_text("today.md").click();
    assert_eq!(*picked.borrow(), [1]);
}

#[test]
fn a_status_has_a_shape_a_name_and_a_colour() {
    let mut h = mount(|| {
        let marks = [Status::Running, Status::Pending, Status::Terminating, Status::Unknown].map(status_mark);
        row()
            .child(status_label(Status::Failed, ""))
            .child(status_label(Status::Succeeded, "Done"))
            .children(marks)
    });
    assert!(h.has_text("Failed") && h.has_text("Done"));
    let failed = h.get_by_text("Failed").node();
    assert_eq!(h.tree.text_values(failed).color, Some(dark().theme().resolve(token::FAILED.into())));
    assert_eq!(
        ["Running", "Completed", "CrashLoopBackOff", "Restarted"].map(Status::of),
        [Status::Running, Status::Succeeded, Status::Failed, Status::Unknown]
    );
    // The reasons a container is down, as a cluster writes them.
    let failed = ["OOMKilled", "ImagePullBackOff", "ErrImagePull", "CreateContainerConfigError", "Evicted"];
    assert_eq!(failed.map(Status::of), [Status::Failed; 5]);
    let pending = ["ContainerCreating", "PodInitializing", "Init:0/2"];
    assert_eq!(pending.map(Status::of), [Status::Pending; 3]);
    assert_eq!(["Init:CrashLoopBackOff", "Init:Error"].map(Status::of), [Status::Failed; 2]);
}

#[test]
fn a_place_of_a_sidebar_is_chosen_by_a_click_or_the_arrows_and_the_sidebar_folds() {
    let (chosen, folded) = (signal("inbox"), signal(false));
    let mut h = mount(move || {
        let places = sidebar(chosen)
            .group("", [NavItem::new("inbox", "Inbox").icon(icons::BELL)])
            .group("Notes", [NavItem::new("today", "Today"), NavItem::new("all", "All notes")])
            .folded(move || folded.get());
        row().grow(1.0).child(places).child(div().grow(1.0))
    });
    let tree = h.semantics();
    assert!(tree.contains("TreeItem \"Inbox\" [selected]") && tree.contains("Text \"Notes\""), "{tree}");
    // 240 px wide, as tall as the window; a place is 26 px, and filled when chosen.
    let (side, inbox) =
        (h.query(|n| n.semantics.role == Role::Tree)[0], h.get_by_role(Role::TreeItem, "Inbox").node());
    assert_eq!(h.tree.rect(side).size(), Size::new(240.0, 600.0));
    assert_eq!(h.tree.rect(inbox).size(), Size::new(239.0, 26.0)); // less the line at its edge
    assert_eq!(h.tree.paint_style(inbox).background, dark().theme().resolve(token::PRESS.into()));

    h.get_by_role(Role::TreeItem, "All notes").click();
    assert_eq!(chosen.get_untracked(), "all");
    h.key(Key::Up, Mods::NONE); // the press gave the sidebar the focus
    h.key(Key::Home, Mods::NONE);
    h.key(Key::Down, Mods::NONE);
    h.key(Key::End, Mods::NONE);
    h.key(Key::Down, Mods::NONE);
    assert_eq!(chosen.get_untracked(), "all");
    h.key(Key::Up, Mods::NONE);
    settle(&mut h);
    assert!(chosen.get_untracked() == "today" && h.semantics().contains("TreeItem \"Today\" [selected]"));

    // Folded: 40 px, the icons alone; the places keep their names for a reader.
    folded.set(true);
    settle(&mut h);
    let tree = h.semantics();
    assert_eq!(h.tree.rect(side).width, 40.0);
    assert!(tree.contains("TreeItem \"Inbox\"") && !tree.contains("Text \"Notes\""), "{tree}");
}

#[test]
fn a_sidebars_search_keeps_the_places_it_names_and_enter_chooses_the_first() {
    let (chosen, query) = (signal("inbox"), signal(String::new()));
    let mut h = mount(move || {
        let places = sidebar(chosen)
            .search(query)
            .group("", [NavItem::new("inbox", "Inbox").icon(icons::BELL)])
            .group("Notes", [NavItem::new("today", "Today"), NavItem::new("all", "All notes")]);
        row().grow(1.0).child(places).child(div().grow(1.0))
    });
    let shown = |h: &Harness| {
        let rows = h.query(|n| n.semantics.role == Role::TreeItem);
        rows.into_iter()
            .filter(|&id| h.tree.rect(id).height > 0.0)
            .map(|id| h.tree.meaning(id).name)
            .collect::<Vec<_>>()
    };
    assert_eq!(shown(&h), ["Inbox", "Today", "All notes"]);
    let field = h.get_by_role(Role::TextInput, "Search").node();
    let frame = h.tree.parent(field).expect("the field's frame");
    assert_eq!(h.tree.rect(frame).height, 28.0);
    h.get_by_role(Role::TextInput, "Search").type_text("NOTE");
    settle(&mut h);
    assert_eq!(shown(&h), ["All notes"], "any case");
    assert!(h.has_text("Notes"), "its group's title stays");
    h.key(Key::Enter, Mods::NONE);
    settle(&mut h);
    assert_eq!(chosen.get_untracked(), "all");
    query.set("inb".into());
    settle(&mut h);
    assert_eq!(shown(&h), ["Inbox"]);
    assert!(!h.has_text("Notes"), "a group with no place kept has no title");
    query.set("zzz".into());
    settle(&mut h);
    assert!(shown(&h).is_empty() && h.has_text("No results"));
    // The arrows go among the places kept: "n" keeps Inbox and All notes, not Today.
    query.set("n".into());
    chosen.set("inbox");
    settle(&mut h);
    assert_eq!(shown(&h), ["Inbox", "All notes"]);
    h.get_by_role(Role::TreeItem, "Inbox").click();
    settle(&mut h);
    h.key(Key::Down, Mods::NONE);
    settle(&mut h);
    assert_eq!(chosen.get_untracked(), "all", "Today is passed over");
    h.key(Key::Home, Mods::NONE);
    settle(&mut h);
    assert_eq!(chosen.get_untracked(), "inbox");
}

#[test]
fn a_place_shows_its_count_and_the_foot_stays_under_the_places() {
    let (chosen, folded, pods, query) =
        (signal("pods"), signal(false), signal(420usize), signal(String::new()));
    let mut h = mount(move || {
        let items = [
            NavItem::new("pods", "Pods"),
            NavItem::new("secrets", "Secrets"),
            NavItem::new("events", "Events"),
        ];
        let places = sidebar(chosen)
            .search(query)
            .group("", items)
            .counts(move |id| match id {
                "pods" => Some(pods.get()),
                "secrets" => Some(148),
                _ => None,
            })
            .foot(quadra_design::input(signal(String::new())).label("Filter by pod name"))
            .folded(move || folded.get());
        row().grow(1.0).child(places).child(div().grow(1.0))
    });
    // The count is part of the place's name for a reader; a place without one has none.
    let tree = h.semantics();
    assert!(tree.contains("TreeItem \"Pods, 420\"") && tree.contains("TreeItem \"Events\""), "{tree}");
    assert!(h.has_text("148"));
    pods.set(421);
    settle(&mut h);
    assert!(h.semantics().contains("TreeItem \"Pods, 421\"") && h.has_text("421"));
    // The search reads the name alone.
    query.set("42".into());
    settle(&mut h);
    assert!(h.has_text("No results"));
    query.set(String::new());
    settle(&mut h);
    // The foot is at the bottom of the sidebar, under the places.
    let side = h.query(|n| n.semantics.role == Role::Tree)[0];
    let field = h.get_by_role(Role::TextInput, "Filter by pod name").node();
    let (side, field) = (h.tree.window_rect(side), h.tree.window_rect(field));
    assert!(field.y > side.y + side.height - 60.0, "{field:?} in {side:?}");
    // Folded: no count, no foot.
    folded.set(true);
    settle(&mut h);
    let foot = h.query(|n| n.semantics.name == "Filter by pod name");
    assert!(!h.has_text("421") && foot.is_empty(), "{}", h.semantics());
}
