//! Tabs and the list (specs/13, "Components"): one of a few names in a row, and one of any
//! number of rows. Each writes the signal that holds the choice, by the pointer or the keys.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_kit::{dark, list, tabs, token};
use quadra_platform::{Key, Mods};
use quadra_prim::Color;
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_ui::{column, div, text};

struct Page {
    h: Harness,
    tab: Signal<usize>,
    view: Signal<usize>,
    count: Signal<usize>,
    row: Signal<Option<usize>>,
    opened: Rc<RefCell<Vec<usize>>>,
}

/// Underlined tabs, pill tabs, and a list of a thousand rows in 230 px: ten rows show.
fn page() -> Page {
    let (tab, view, count, row) = (signal(0usize), signal(1usize), signal(1000usize), signal(None));
    let opened: Rc<RefCell<Vec<usize>>> = Rc::default();
    let log = opened.clone();
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_kit::keymap()));
    h.mount(move || {
        let files = list(move || count.get(), row, |i| text(move || format!("Row {}", i.get())))
            .on_activate(move |at| log.borrow_mut().push(at))
            .empty("Nothing to show");
        let pages = tabs(["Files", "Search", "History"], tab);
        let views = tabs(["List", "Grid"], view).pill();
        let frame = div().height(230.0).child(files);
        quadra_kit::root().child(column().child(pages).child(views).child(frame))
    });
    h.frame();
    Page { h, tab, view, count, row, opened }
}

impl Page {
    fn settle(&mut self) {
        self.h.frame();
        while self.h.frame_if_needed().is_some() {}
    }

    fn key(&mut self, key: Key) {
        self.h.key(key, Mods::NONE);
        self.settle();
    }

    fn fill(&mut self, role: Role, name: &str) -> Color {
        let node = self.h.get_by_role(role, name).node();
        self.h.tree.paint_style(node).background
    }
}

#[test]
fn tabs_are_chosen_by_a_click_or_the_arrows() {
    let mut p = page();
    let colour = |t| dark().theme().resolve(quadra_tree::Paint::from(t));
    let tree = p.h.semantics();
    assert!(tree.contains("Tab \"Files\" [selected]") && tree.contains("Tab \"Grid\" [selected]"), "{tree}");
    // A tab is 28 px tall; the line under the row of underlined tabs makes it 29.
    let (files, strip) =
        (p.h.get_by_role(Role::Tab, "Files").node(), p.h.query(|n| n.semantics.role == Role::TabList)[0]);
    assert_eq!((p.h.tree.rect(files).height, p.h.tree.rect(strip).height), (28.0, 29.0));
    // The chosen pill is tinted; the chosen underlined tab is not, whatever the pointer does.
    assert_eq!(
        (p.fill(Role::Tab, "Grid"), p.fill(Role::Tab, "Files")),
        (colour(token::ACCENT_TINT), Color::TRANSPARENT)
    );

    p.h.get_by_role(Role::Tab, "History").click();
    p.settle();
    assert_eq!(p.tab.get_untracked(), 2);
    assert!(p.h.semantics().contains("Tab \"History\" [selected]"));
    // The row of tabs is one stop of Tab; the arrows go round its ends.
    p.h.tree.focus(strip);
    p.key(Key::Right);
    assert_eq!(p.tab.get_untracked(), 0);
    p.key(Key::Left);
    assert_eq!((p.tab.get_untracked(), p.view.get_untracked()), (2, 1));
}

#[test]
fn a_row_of_a_list_is_selected_by_a_click_or_the_keys_and_kept_in_view() {
    let mut p = page();
    // Ten rows of 23 px show, and a few more are kept ready: not a thousand.
    let rows = p.h.query(|n| n.semantics.role == Role::ListItem).len();
    assert!((10..=14).contains(&rows), "{rows}");
    p.h.get_by_role(Role::ListItem, "Row 3").click();
    p.settle();
    assert_eq!(p.row.get_untracked(), Some(3));
    assert!(p.h.semantics().contains("ListItem \"Row 3\" [selected]"));
    let (line, colour) = (p.h.get_by_role(Role::ListItem, "Row 3").node(), dark().theme());
    // A row is 23 px tall and as wide as the list.
    assert_eq!(p.h.tree.rect(line).size(), quadra_prim::Size::new(800.0, 23.0));
    assert_eq!(p.h.tree.paint_style(line).background, colour.resolve(token::PRESS.into()));

    // The press gave the list the focus: the arrows move the selection and stop at the ends.
    p.key(Key::Down);
    p.key(Key::Down);
    p.key(Key::Up);
    assert_eq!(p.row.get_untracked(), Some(4));
    p.key(Key::End);
    assert_eq!(p.row.get_untracked(), Some(999));
    let last = p.h.get_by_role(Role::ListItem, "Row 999").node(); // it was brought into view
    let (line, frame) = (p.h.tree.window_rect(last), p.h.query(|n| n.semantics.role == Role::List)[0]);
    assert!(line.bottom() <= p.h.tree.window_rect(frame).bottom() + 0.5, "{line:?}");
    p.key(Key::Down);
    assert_eq!(p.row.get_untracked(), Some(999));
    p.key(Key::Home);
    p.key(Key::Up);
    assert_eq!(p.row.get_untracked(), Some(0));

    // Enter and a double click choose a row for good.
    p.key(Key::Enter);
    let second = p.h.get_by_role(Role::ListItem, "Row 2").node();
    let at = p.h.tree.window_rect(second).center();
    p.h.click(at.x, at.y);
    p.h.click(at.x, at.y);
    p.settle();
    assert_eq!(*p.opened.borrow(), [0, 2]);

    // A list with no rows says so.
    assert!(!p.h.has_text("Nothing to show"));
    p.count.set(0);
    p.settle();
    assert!(p.h.has_text("Nothing to show") && p.h.query(|n| n.semantics.role == Role::ListItem).is_empty());
}
