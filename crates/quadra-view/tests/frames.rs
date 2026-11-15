//! Frame callbacks (specs/05): app code advanced by the frame. A callback runs before the
//! update pass of each frame, the frame shows what it wrote, and it ends by returning false
//! or with its scope.

use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::Text;
use quadra_view::{column, each_frame, mount_with, show, text};

#[test]
fn a_callback_runs_each_frame_until_it_says_stop() {
    let count = signal(0);
    let mut h = Harness::new();
    mount_with(&mut h.tree, move || {
        each_frame(move || {
            count.update(|n| *n += 1);
            count.get_untracked() < 3
        });
        column().child(text(move || count.get().to_string()))
    });
    let label = h.tree.children(h.tree.root().unwrap())[0];
    for expected in ["1", "2", "3"] {
        assert!(h.tree.needs_frame()); // a window with a callback wants every frame
        let (stats, _) = h.frame();
        // The frame shows what its callback wrote. After the first: one binding, one text.
        assert_eq!(h.tree.widget_mut::<Text>(label).unwrap().text(), expected);
        if expected != "1" {
            assert_eq!((stats.bindings_run, stats.nodes_painted), (1, 1));
        }
    }
    assert!(!h.tree.animating() && !h.tree.needs_frame()); // it returned false: asleep again
    assert!(h.frame_if_needed().is_none());
}

#[test]
fn a_callback_ends_with_its_scope_and_another_may_start_while_one_runs() {
    let (shown, ticks, late) = (signal(true), signal(0), signal(0));
    let mut h = Harness::new();
    mount_with(&mut h.tree, move || {
        column().child(show(
            move || shown.get(),
            move || {
                each_frame(move || {
                    ticks.update(|n| *n += 1);
                    if ticks.get_untracked() == 2 {
                        // Registered from inside a callback: it starts with the next frame.
                        each_frame(move || {
                            late.update(|n| *n += 1);
                            true
                        });
                    }
                    true
                });
                text("on")
            },
        ))
    });
    for _ in 0..3 {
        h.frame();
    }
    assert_eq!((ticks.get_untracked(), late.get_untracked()), (3, 1));
    shown.set(false);
    // Callbacks run before the update pass, which is where the part goes: once more, then
    // both end with its scope.
    h.frame();
    assert_eq!((ticks.get_untracked(), late.get_untracked()), (4, 2));
    h.frame();
    assert_eq!((ticks.get_untracked(), late.get_untracked()), (4, 2));
    assert!(!h.tree.animating());
}
