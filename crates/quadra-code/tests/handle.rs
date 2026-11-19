//! Reaching the editor from outside it (specs/13, "Code"): its version told after each change
//! of the text, read and edited through its handle as a key would, and nothing done once it
//! is gone.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_code::{EditorHandle, editor};
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_view::{column, show};

#[test]
fn a_handle_reads_and_edits_and_the_version_moves_with_the_text() {
    let (version, shown, reach) = (signal(0u64), signal(true), EditorHandle::default());
    let mut h = Harness::new();
    let given = reach.clone();
    h.mount(move || {
        let code = move || editor("one\ntwo\n").edited(version).handle(given.clone());
        column().size(400.0, 200.0).child(show(move || shown.get(), code))
    });
    h.settle();
    assert!(reach.is_mounted());
    assert_eq!(version.get_untracked(), 0, "nothing changed yet");
    // An edit from outside: the text, the version, and what a reader is told.
    reach.edit(|w| w.doc.insert_text("zero\n", true));
    h.settle();
    assert!(version.get_untracked() > 0);
    let read = Rc::new(RefCell::new(String::new()));
    let into = read.clone();
    reach.read(move |w| *into.borrow_mut() = w.doc.text());
    h.settle();
    assert_eq!(*read.borrow(), "zero\none\ntwo\n");
    let value = h.query(|n| n.semantics.role == Role::TextInput).first().map(|&id| h.tree.meaning(id).value);
    assert_eq!(value.flatten().as_deref(), Some("zero\none\ntwo\n"));
    // Gone, it is not reached.
    shown.set(false);
    h.settle();
    assert!(!reach.is_mounted());
}
