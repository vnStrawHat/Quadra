//! A node keeps its room (specs/07, "Display lists"): a count of instances that shrinks fills
//! the rest with empty ones, so the layer is patched in place, not rebuilt; what is hidden or
//! culled keeps none.

use quadra_layout::Display;
use quadra_prim::Color;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_view::{column, div, text};

/// A number whose digits come and go, among `n` others: after it first reaches its longest,
/// every change overwrites its own range.
fn one_number_among(n: usize) {
    let value = signal("9.99".to_string());
    let mut h = Harness::new();
    h.mount(move || {
        let others = (0..n).map(|i| text(format!("{i}")).size(10.0));
        column().children(others).child(text(value).size(10.0))
    });
    h.frame();
    value.set("10.00".to_string()); // longer: the layer is rebuilt once
    assert_eq!(h.frame().0.layers_rebuilt, 1);
    for shown in ["9.99", "10.00", "0.01", "99.99"] {
        value.set(shown.to_string());
        let (frame, drawn) = h.frame();
        assert_eq!((frame.nodes_painted, frame.layers_patched, frame.layers_rebuilt), (1, 1, 0));
        assert_eq!(drawn.instances_uploaded, 5, "{shown}"); // its range, whatever the page holds
    }
}

#[test]
fn a_number_whose_digits_come_and_go_is_patched_in_place() {
    one_number_among(10);
    one_number_among(5_000);
}

#[test]
fn a_hidden_box_keeps_no_room() {
    let (hidden, top) = (signal(false), signal(10.0f32));
    let mut h = Harness::new();
    h.mount(move || {
        let shown = move || if hidden.get() { Display::None } else { Display::Flex };
        let boxed = div().size(20.0, 20.0).bg(Color::rgb(0x336699)).display(shown);
        column().child(div().width(10.0).height(top)).child(column().child(boxed))
    });
    h.frame();
    hidden.set(true);
    h.frame();
    top.set(20.0); // what holds the hidden box moves, and is painted again
    let (frame, _) = h.frame();
    assert_eq!(frame.layers_rebuilt, 0, "{frame:?}");
}
