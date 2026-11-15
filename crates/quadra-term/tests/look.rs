//! The look as a signal (specs/18, "The view"): a new size, line or room sizes the grid again
//! without mounting the view again.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_reactive::signal;
use quadra_term::vt::ResizePolicy;
use quadra_term::{Session, TermLook, terminal};
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_view::column;

#[test]
fn a_new_look_sizes_the_grid_again() {
    let mut h = Harness::new();
    let look = signal(TermLook { size: 10.0, line_height: 1.4, ..TermLook::default() });
    let slot = Rc::new(RefCell::new(None));
    let out = slot.clone();
    h.mount(move || {
        let s = Session::new(ResizePolicy::BottomAnchor);
        *out.borrow_mut() = Some(s.clone());
        column().size(400.0, 140.0).child(terminal(&s).look(look))
    });
    h.settle();
    let s: Session = slot.borrow_mut().take().unwrap();
    let node = h.query(|n| n.semantics.role == Role::TextInput)[0];
    let rows = |s: &Session| s.size().rows;
    assert_eq!(rows(&s), 8, "(140 - 16) / 14");
    look.update(|it| it.size = 20.0);
    h.settle();
    assert_eq!(rows(&s), 4, "(140 - 16) / 28");
    look.update(|it| it.padding = 20.0);
    h.settle();
    assert_eq!(rows(&s), 3, "(140 - 40) / 28");
    assert_eq!(h.query(|n| n.semantics.role == Role::TextInput)[0], node, "the same view");
}
