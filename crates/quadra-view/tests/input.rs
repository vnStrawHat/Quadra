//! Handlers on views (specs/05, "Handlers"; specs/09): a click runs a handler, the handler
//! writes a signal, and the next frame runs the one binding that reads it.

use quadra_prim::Color;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::{PointerInput, PointerKind, Text};
use quadra_view::{column, div, mount_root, row, show, text};

#[test]
fn a_counter_counts_its_clicks() {
    let count = signal(0);
    let button = |label: &str, by: i32| {
        div().size(40.0, 20.0).child(text(label.to_string())).on_click(move |_| count.update(|n| *n += by))
    };
    let number = text(move || count.get().to_string());
    let mut h = Harness::new();
    mount_root(&mut h.tree, column().child(number).child(row().child(button("-", -1)).child(button("+", 1))));
    h.frame();
    let label = h.tree.children(h.tree.root().unwrap())[0];
    let shows = |h: &mut Harness| h.tree.widget_mut::<Text>(label).unwrap().text().to_string();

    // The number's line is about 20 px high; the buttons are below it, 40 px each.
    h.click(60.0, 30.0);
    h.click(60.0, 30.0);
    let (stats, _) = h.frame(); // two clicks before a frame: one binding run, not two
    assert_eq!((shows(&mut h), stats.bindings_run), ("2".to_string(), 1));
    h.click(10.0, 30.0);
    h.frame();
    assert_eq!(shows(&mut h), "1");
    h.click(300.0, 300.0); // beside the buttons
    assert!(h.frame_if_needed().is_none());
}

#[test]
fn a_pointer_over_plain_content_asks_for_no_frame_and_hover_drives_what_reads_it() {
    let over = signal(false);
    let card = div().size(100.0, 50.0).bg(Color::rgb(0x202020));
    let card = card.on_pointer(PointerKind::Enter, move |_| over.set(true));
    let card = card.on_pointer(PointerKind::Leave, move |_| over.set(false));
    let hint = show(move || over.get(), || text("over"));
    let mut h = Harness::new();
    mount_root(&mut h.tree, column().child(card).child(div().size(100.0, 50.0)).child(hint));
    h.frame();

    h.pointer(PointerInput::Move, 50.0, 75.0); // the box without handlers
    h.pointer(PointerInput::Move, 60.0, 80.0);
    assert!(h.frame_if_needed().is_none()); // specs/14: no frame

    h.pointer(PointerInput::Move, 50.0, 25.0);
    let (stats, _) = h.frame();
    assert!(over.get_untracked() && stats.nodes_painted > 0); // the hint appears
    h.pointer(PointerInput::Leave, 0.0, 0.0);
    h.frame();
    assert!(!over.get_untracked());
}

#[test]
fn a_box_shows_hover_and_press_without_running_app_code() {
    let (base, over, down) = (Color::rgb(0xE8E8E8), Color::rgb(0xD8D8D8), Color::rgb(0xC0C0C0));
    let ran = signal(0);
    let button = move || {
        ran.update(|n| *n += 1);
        div().size(80.0, 30.0).bg(base).radius(6.0).hover(|s| s.bg(over)).active(|s| s.bg(down).radius(4.0))
    };
    let mut h = Harness::new();
    mount_root(&mut h.tree, column().child(button()));
    h.frame();
    let node = h.tree.children(h.tree.root().unwrap())[0];
    h.pointer(PointerInput::Move, 40.0, 15.0);
    let (stats, _) = h.frame();
    assert_eq!((h.tree.paint_style(node).background, stats.bindings_run, stats.nodes_styled), (over, 0, 1));
    h.pointer(PointerInput::Down, 40.0, 15.0);
    h.frame();
    assert_eq!((h.tree.paint_style(node).background, h.tree.paint_style(node).radii.top_left), (down, 4.0));
    h.pointer(PointerInput::Up, 40.0, 15.0);
    h.pointer(PointerInput::Move, 400.0, 300.0);
    h.frame();
    assert_eq!((h.tree.paint_style(node).background, ran.get_untracked()), (base, 1)); // built once
}

#[test]
fn the_wheel_scrolls_what_is_under_the_pointer_and_what_is_left_goes_outwards() {
    use quadra_view::{scroll, uniform_list};
    let (outer_y, inner_y, list_y) = (signal(0.0f32), signal(0.0f32), signal(0.0f32));
    // A page that scrolls, with a small scroll area and a virtual list in it.
    let inner = scroll(inner_y, div().size(200.0, 300.0)).container(div().size(200.0, 100.0));
    let rows = uniform_list(|| 1000, 20.0, list_y, |i| text(move || i.get().to_string()));
    let page = column()
        .child(inner)
        .child(rows.container(div().size(200.0, 200.0)))
        .child(div().size(200.0, 2000.0));
    let mut h = Harness::new();
    mount_root(&mut h.tree, scroll(outer_y, page));
    h.frame();

    h.wheel(50.0, 50.0, -1.0); // one line towards the end, over the small area
    assert_eq!((inner_y.get_untracked(), outer_y.get_untracked()), (40.0, 0.0));
    let (stats, drawn) = h.frame();
    // The content moved by its layer; what was painted is the thumb of its bar, which shows.
    assert_eq!((stats.nodes_painted, stats.layers_composed, drawn.instances_uploaded), (1, 2, 1));

    // It has 200 px to scroll. Five lines ask for 200 more than the 160 left: the page takes the rest.
    h.wheel(50.0, 50.0, -5.0);
    assert_eq!((inner_y.get_untracked(), outer_y.get_untracked()), (200.0, 40.0));
    h.frame();
    // The page moved up 40 px: the list is under the pointer at 150 + ... and scrolls alone.
    h.wheel(50.0, 100.0, -2.0);
    assert_eq!((list_y.get_untracked(), outer_y.get_untracked()), (80.0, 40.0));
    h.frame();
    h.wheel(50.0, 100.0, 3.0); // back past its start: 80 of the 120 are its own, 40 the page's
    assert_eq!((list_y.get_untracked(), outer_y.get_untracked()), (0.0, 0.0));
    h.frame();
    h.wheel(50.0, 100.0, 1.0); // nothing can scroll further: nothing changes, no frame
    assert!(h.frame_if_needed().is_none());
}

#[test]
fn a_form_is_walked_with_tab_and_its_fields_hear_keys() {
    use quadra_platform::{Key, Mods};
    use quadra_view::NodeRef;
    let (typed, ring) = (signal(String::new()), Color::rgb(0x3366FF));
    let field = |name: &'static str| {
        let on_key = move |e: &mut quadra_tree::KeyEvent| {
            typed.update(|t| t.push_str(&format!("{name}:{:?} ", e.key)))
        };
        div().size(100.0, 20.0).focusable().on_key_down(on_key).focus_visible(|s| s.border_color(ring))
    };
    let second = NodeRef::new();
    let form = column().child(field("a")).child(div().size(100.0, 20.0)).child(field("b").node_ref(&second));
    let mut h = Harness::new();
    mount_root(&mut h.tree, form);
    h.frame();
    let nodes = h.tree.children(h.tree.root().unwrap()).to_vec();
    assert_eq!(second.get(), Some(nodes[2]));

    h.key(Key::Tab, Mods::NONE);
    h.key(Key::X, Mods::NONE);
    let (stats, _) = h.frame();
    // The X went to the field the Tab had focused. One node was styled: it shows its ring.
    assert_eq!((typed.get_untracked().as_str(), stats.nodes_styled), ("a:X ", 1));
    assert_eq!(h.tree.paint_style(nodes[0]).border_color, ring);
    h.key(Key::Tab, Mods::NONE); // past the box that cannot have the focus
    h.key(Key::Y, Mods::NONE);
    h.frame();
    // A key is heard where the focus is. The second Tab was not a key at all: the keymap
    // made it the action that moves the focus.
    assert_eq!(typed.get_untracked(), "a:X b:Y ");
    assert_eq!(
        (h.tree.focused(), h.tree.paint_style(nodes[0]).border_color),
        (Some(nodes[2]), Color::default())
    );

    h.click(50.0, 10.0); // a click focuses without the ring
    h.frame();
    assert_eq!(
        (h.tree.focused(), h.tree.paint_style(nodes[0]).border_color),
        (Some(nodes[0]), Color::default())
    );
    second.focus(); // from app code: in the next frame
    assert_eq!(h.tree.focused(), Some(nodes[0]));
    h.frame();
    assert_eq!(h.tree.focused(), Some(nodes[2]));
}
