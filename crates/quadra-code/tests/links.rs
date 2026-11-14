//! Links in the editor (specs/13, "Code"): Ctrl and a press open one and move no cursor,
//! and a hint over one says where it goes. On the fake backend at size 10 a cell is 6 px.

use std::time::Duration;

use quadra_code::{EditorWidget, editor};
use quadra_platform::Mods;
use quadra_testing::Harness;
use quadra_tree::{NodeId, PointerInput, Role};
use quadra_ui::column;

struct Ed {
    h: Harness,
    id: NodeId,
}

impl Ed {
    fn w(&mut self) -> &mut EditorWidget {
        self.h.frame_if_needed();
        self.h.tree.widget_mut::<EditorWidget>(self.id).unwrap()
    }

    fn head(&mut self) -> usize {
        self.w().doc.primary().head
    }
}

#[test]
fn ctrl_and_a_press_on_a_link_opens_it_and_moves_no_cursor() {
    use quadra_platform::MouseButton;
    use quadra_prim::Point;
    let opened = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let seen = opened.clone();
    let mut h = Harness::new();
    let text = "see www.example.com/a now".to_string();
    h.mount(move || {
        let on = editor(text).font(10.0, 14.0).on_link(move |url| seen.borrow_mut().push(url.to_string()));
        column().size(400.0, 140.0).child(on)
    });
    h.frame();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    let at = Point::new(54.0 + 8.0 * 6.0, 7.0); // on "example"
    let now = h.now();
    h.tree.pointer(PointerInput::Move, MouseButton::None, at, Mods::CTRL, now);
    h.frame();
    let mut e = Ed { h, id };
    assert_eq!(e.h.tree.cursor(), quadra_platform::Cursor::Pointer); // the pointer of a link
    e.h.tree.pointer(PointerInput::Down, MouseButton::Left, at, Mods::CTRL, now);
    e.h.tree.pointer(PointerInput::Up, MouseButton::Left, at, Mods::CTRL, now);
    e.h.frame();
    assert_eq!(*opened.borrow(), vec!["https://www.example.com/a".to_string()]);
    assert_eq!((e.head(), e.w().doc.cursors.len()), (0, 1));
    // Without Ctrl it is text like any other: the press puts the cursor.
    e.h.advance(Duration::from_secs(1));
    e.h.click(at.x, at.y);
    assert_eq!(e.head(), 8);
    // Once the pointer has rested on it, a hint says where it goes and how to open it.
    e.h.pointer(PointerInput::Move, 10.0, 7.0);
    e.h.frame();
    e.h.pointer(PointerInput::Move, at.x, at.y);
    e.h.frame();
    e.h.advance(Duration::from_secs(1));
    e.h.settle();
    let tip = e.h.query(|n| n.semantics.role == Role::Tooltip);
    assert_eq!(tip.len(), 1, "{}", e.h.semantics());
    assert!(e.h.has_text("https://www.example.com/a (Ctrl-click to open)"));
    e.h.pointer(PointerInput::Move, 10.0, 7.0);
    e.h.settle();
    assert!(e.h.query(|n| n.semantics.role == Role::Tooltip).is_empty());
}
