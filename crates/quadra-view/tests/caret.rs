//! The caret's blink (specs/14, "Caret blink"): a timer of the focused input turns the caret
//! off and on, each turn overwriting one instance in place. A caret in use stays shown, and
//! an input without the focus has no timer.

use std::time::Duration;

use quadra_platform::{Key, Mods};
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::TextField;
use quadra_view::{column, div, text_input};

const TURN: Duration = Duration::from_millis(530);

#[test]
fn the_caret_blinks_in_place_while_the_input_has_the_focus() {
    let name = signal("abc".to_string());
    let mut h = Harness::new();
    h.mount(move || column().child(div().size(120.0, 20.0).child(text_input(name).size(10.0))));
    h.frame();
    let field = h.tree.children(h.tree.children(h.tree.root().unwrap())[0])[0];
    let shown = |h: &mut Harness| h.tree.widget_mut::<TextField>(field).unwrap().caret_on;

    h.advance(TURN * 4); // without the focus nothing blinks, and nothing asks for a frame
    assert!(h.frame_if_needed().is_none());

    h.click(10.0, 10.0);
    h.frame();
    assert!(shown(&mut h));
    h.advance(TURN); // used a moment ago: it stays shown for this turn
    h.frame_if_needed();
    assert!(shown(&mut h));
    for expected in [false, true, false] {
        h.advance(TURN);
        let (stats, drawn) = h.frame();
        assert_eq!(shown(&mut h), expected);
        // One node painted, its range overwritten: the caret's instance and its neighbours.
        assert_eq!((stats.nodes_painted, stats.layers_patched, stats.layers_rebuilt), (1, 1, 0));
        assert_eq!((stats.nodes_laid_out, drawn.layers_uploaded), (0, 1));
    }
    h.key(Key::Left, Mods::NONE); // the caret is used: shown at once, and through the next turn
    h.frame();
    assert!(shown(&mut h));
    h.advance(TURN);
    h.frame_if_needed();
    assert!(shown(&mut h));

    h.click(400.0, 300.0); // the focus leaves: the timer ends with it
    h.frame();
    h.advance(TURN * 4);
    assert!(h.frame_if_needed().is_none());
}
