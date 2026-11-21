//! The headless crate's popups with bare parts (specs/17, "Tests"): a menu with a menu in a row, what
//! is picked by typing, a combobox, a tooltip, notifications.

use std::rc::Rc;
use std::time::Duration;

use quadra_headless::menu::{self, Ends, Line, Row};
use quadra_headless::pick::{self, Found, Parts};
use quadra_headless::{combo, hint, notices, popup};
use quadra_platform::{Key, Mods};
use quadra_reactive::{memo, signal};
use quadra_testing::Harness;
use quadra_tree::{Along, Anchor, PointerInput, Role, Side};
use quadra_view::{AnyView, Div, NodeRef, Place, View, column, div, text, text_input};

fn mounted<V: View + 'static>(view: impl Fn() -> V + 'static) -> Harness {
    let mut h = Harness::new();
    h.tree.set_keymap(Rc::new(quadra_headless::keymap()));
    h.mount(view);
    settle(&mut h);
    h
}

fn settle(h: &mut Harness) {
    h.frame();
    while h.frame_if_needed().is_some() {}
}

fn picker(chosen: quadra_reactive::Signal<Vec<&'static str>>, what: &'static str) -> Rc<dyn Fn()> {
    Rc::new(move || chosen.update(|all| all.push(what)))
}

fn row_of(label: &str) -> Row {
    Row::new(div().height(20.0).child(text(label.to_string())), label)
}

#[test]
fn a_menu_moves_its_highlight_opens_a_row_s_menu_and_ends_on_a_choice() {
    let (done, chosen) = (signal(0), signal(Vec::new()));
    let mut h = mounted(move || {
        let pick = move |what: &'static str| picker(chosen, what);
        let mut copy = row_of("Copy");
        copy.select = Some(pick("copy"));
        let mut off = row_of("Off");
        off.disabled = true;
        let mut more = row_of("More");
        let inner: menu::Inner = Rc::new(move |at: &NodeRef, shown: popup::Shown, ends: Ends| {
            let place = Place::Beside(at.clone(), Side::Right, Along::Start, 0.0);
            let rows = move || {
                let mut deep = row_of("Deep");
                deep.select = Some(pick("deep"));
                menu::rows(column(), vec![Line::Row(Box::new(deep))], ends.clone())
            };
            popup::popup(shown, place, Some(at.clone()), column, rows)
        });
        more.inner = Some(inner);
        let lines = vec![
            Line::Row(Box::new(copy)),
            Line::Part(Box::new(div().height(5.0))),
            Line::Row(Box::new(off)),
            Line::Row(Box::new(more)),
        ];
        menu::rows(
            column().width(100.0),
            lines,
            Ends { done: Rc::new(move || done.update(|n| *n += 1)), leave: None },
        )
    });
    let list = h.get_by_role(Role::Menu, "").node();
    h.tree.focus(list);
    // Down: Copy; down again: past the line and the disabled row to More; round to Copy.
    h.key(Key::Down, Mods::NONE);
    h.key(Key::Down, Mods::NONE);
    settle(&mut h);
    assert!(h.semantics().contains("MenuItem \"More\" [selected]"), "{}", h.semantics());
    h.key(Key::Right, Mods::NONE); // opens the menu of More
    settle(&mut h);
    assert!(h.semantics().contains("MenuItem \"Deep\""), "{}", h.semantics());
    h.key(Key::Down, Mods::NONE);
    h.key(Key::Enter, Mods::NONE); // a choice in it ends the menus together
    settle(&mut h);
    assert_eq!((done.get_untracked(), chosen.get_untracked()), (1, vec!["deep"]));
    h.get_by_role(Role::MenuItem, "Copy").click();
    assert_eq!((done.get_untracked(), chosen.get_untracked()), (2, vec!["deep", "copy"]));
}

#[test]
fn typing_narrows_what_is_found_and_the_best_match_is_first() {
    assert_eq!(pick::fuzzy_hits("hai ph", "Hải Phòng"), Some(vec![0, 1, 2, 3, 4, 5]));
    assert_eq!(pick::best("ap", ["Map", "Apple", "Paper"].into_iter()), vec![1, 0, 2]);
    let (query, taken) = (signal(String::new()), signal(None));
    let texts = Rc::new(vec!["Open".to_string(), "Close".to_string(), "Copy".to_string()]);
    let mut h = mounted(move || {
        let known = texts.clone();
        let found = Found::new(memo(move || pick::best(&query.get(), known.iter().map(String::as_str))));
        let (labels, matches) = (texts.clone(), found.matches);
        let parts = Parts {
            head: None,
            list: Rc::new(column),
            line: None,
            row: Rc::new(move |_, i| div().height(20.0).label(labels[i].clone())),
            empty: Some(Rc::new(|| div().child(text("None")))),
        };
        let take = move |k: usize| taken.set(matches.get().get(k).copied());
        let rows = found.rows(parts, |_| 0, take);
        found.keys(column(), take).child(text_input(query)).child(rows)
    });
    query.set("o".into());
    settle(&mut h);
    assert!(h.semantics().contains("ListItem \"Open\" [selected]"), "{}", h.semantics());
    let field = h.get_by_role(Role::TextInput, "").node();
    h.tree.focus(field);
    h.key(Key::Down, Mods::NONE);
    h.key(Key::Down, Mods::NONE);
    h.key(Key::Down, Mods::NONE); // round the three
    settle(&mut h);
    assert!(h.semantics().contains("ListItem \"Open\" [selected]"));
    h.get_by_role(Role::ListItem, "Close").click();
    assert_eq!(taken.get_untracked(), Some(1));
    query.set("zz".into());
    settle(&mut h);
    assert!(h.semantics().contains("None"));
}

#[test]
fn a_combobox_opens_as_it_is_typed_into_and_takes_an_option() {
    let value = signal(String::new());
    let mut h = mounted(move || {
        let options = Rc::new(vec!["Hà Nội".to_string(), "Hải Phòng".to_string()]);
        let labels = options.clone();
        let parts = Parts {
            head: None,
            list: Rc::new(column),
            line: None,
            row: Rc::new(move |_, i| div().height(20.0).label(labels[i].clone())),
            empty: None,
        };
        let field = move |submit: Rc<dyn Fn()>| text_input(value).on_submit(move |_| submit());
        let panel = |_: &Found, rows: AnyView| column().width(100.0).child(rows);
        combo::combo(options, value, parts, field, panel, |at: &NodeRef| {
            Place::Beside(at.clone(), Side::Below, Along::Start, 0.0)
        })
    });
    let field = h.get_by_role(Role::TextInput, "").node();
    h.tree.focus(field);
    h.type_text("hp");
    settle(&mut h);
    assert!(h.semantics().contains("ListItem \"Hải Phòng\" [selected]"), "{}", h.semantics());
    h.key(Key::Enter, Mods::NONE);
    settle(&mut h);
    assert_eq!(value.get_untracked(), "Hải Phòng");
    assert!(!h.semantics().contains("ListItem"), "taken: the list is closed");
    assert_eq!(h.tree.focused(), Some(field));
}

#[test]
fn a_tooltip_shows_after_a_rest_and_goes_on_a_press_and_notices_go_when_they_last() {
    let queue: Rc<std::cell::Cell<Option<notices::Queue<&'static str>>>> = Rc::default();
    let mut h = Harness::new();
    let made = queue.clone();
    h.mount(move || {
        made.set(Some(notices::Queue::new()));
        let host = div().size(80.0, 20.0).role(Role::Button).label("Host");
        TipHost(host)
    });
    settle(&mut h);
    let host = h.get_by_role(Role::Button, "Host").node();
    let at = h.tree.window_rect(host).center();
    h.pointer(PointerInput::Move, at.x, at.y);
    h.advance(Duration::from_millis(400));
    settle(&mut h);
    assert!(!h.semantics().contains("Tooltip"));
    h.advance(Duration::from_millis(150));
    settle(&mut h);
    assert!(h.semantics().contains("Tooltip \"Tip\""), "{}", h.semantics());
    h.pointer(PointerInput::Down, at.x, at.y);
    settle(&mut h);
    assert!(!h.semantics().contains("Tooltip"));

    let queue = queue.get().expect("made in the view");
    let (first, _) = (queue.push("first", None), queue.push("second", Some(Duration::from_secs(1))));
    assert_eq!(queue.ids(|_| true).len(), 2);
    h.advance(Duration::from_millis(1100));
    assert_eq!((queue.ids(|_| true), queue.find(first)), (vec![first], Some("first")));
    queue.close(first);
    assert!(queue.ids(|_| true).is_empty());
}

/// A view with a tooltip of bare boxes.
struct TipHost(Div);

impl View for TipHost {
    fn mount(self, cx: &mut quadra_view::MountCx) -> quadra_tree::NodeId {
        let of = self.0.mount(cx);
        let shown = hint::tooltip(cx, of, hint::TOOLTIP_REST);
        let anchor = Anchor::Beside { node: of, side: Side::Below, along: Along::Center, gap: 0.0 };
        popup::hang(cx, of, shown, anchor, || div().role(Role::Tooltip).label("Tip").size(30.0, 10.0));
        of
    }
}
