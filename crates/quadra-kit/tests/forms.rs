//! The controls of a form beyond the text input (specs/behaviour/09, "Forms"): the slider,
//! the number input and its stepper, a field with its label and what is wrong, and the rules
//! a value is checked against.

use std::rc::Rc;

use quadra_kit::{Rule, dark, field, input, number_input, slider, textarea, validate};
use quadra_platform::{Key, Mods};
use quadra_prim::Size;
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role, SemanticAction};
use quadra_ui::{View, column};

fn mount<V: View + 'static>(build: impl FnOnce() -> V + 'static) -> Harness {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_kit::keymap()));
    h.mount(move || quadra_kit::root().child(build()));
    h.frame();
    h
}

fn settle(h: &mut Harness) {
    h.frame();
    while h.frame_if_needed().is_some() {}
}

#[test]
fn a_slider_is_set_by_the_pointer_the_keys_and_a_reader() {
    let volume = signal(20.0f32);
    let mut h = mount(move || column().child(slider(volume).range(0.0, 100.0, 5.0).label("Volume")));
    assert!(h.semantics().contains("Slider \"Volume\" = \"20\""), "{}", h.semantics());
    let face = h.get_by_role(Role::Slider, "Volume").node();
    assert_eq!(h.tree.rect(face).size(), Size::new(200.0, 20.0));
    // The knob is 12 px, a fifth of the way along the track of 184.
    let knob = *h.tree.children(face).last().unwrap();
    assert_eq!((h.tree.rect(knob).x, h.tree.rect(knob).width), (184.0 * 0.2 + 2.0, 12.0));

    // A press on the track takes the value there, to the nearest step; a drag goes on from it
    // wherever the pointer is, and stops at the ends.
    h.pointer(PointerInput::Move, 100.0, 10.0);
    h.pointer(PointerInput::Down, 100.0, 10.0);
    assert_eq!(volume.get_untracked(), 50.0);
    h.pointer(PointerInput::Move, 150.0, 300.0);
    assert_eq!(volume.get_untracked(), 75.0);
    h.pointer(PointerInput::Move, 900.0, 300.0);
    h.pointer(PointerInput::Up, 900.0, 300.0);
    h.pointer(PointerInput::Move, 20.0, 10.0);
    settle(&mut h);
    assert_eq!((volume.get_untracked(), h.tree.rect(knob).x), (100.0, 186.0));

    // The press gave it the focus: the arrows step, the page keys go a tenth, Home and End
    // to the ends.
    for (key, expected) in [(Key::Left, 95.0), (Key::Down, 90.0), (Key::PageDown, 80.0), (Key::Home, 0.0)] {
        h.key(key, Mods::NONE);
        assert_eq!(volume.get_untracked(), expected, "{key:?}");
    }
    for (key, expected) in [(Key::Down, 0.0), (Key::Up, 5.0), (Key::PageUp, 15.0), (Key::End, 100.0)] {
        h.key(key, Mods::NONE);
        assert_eq!(volume.get_untracked(), expected, "{key:?}");
    }
    h.get_by_role(Role::Slider, "Volume").act(SemanticAction::Decrement);
    h.get_by_role(Role::Slider, "Volume").act(SemanticAction::SetValue("42".into()));
    settle(&mut h);
    assert!(volume.get_untracked() == 40.0 && h.semantics().contains("= \"40\""));
    // Its number in its range, for a reader that says it otherwise than as text.
    let range = quadra_tree::RangeValue { value: 40.0, min: 0.0, max: 100.0, step: 5.0 };
    assert_eq!(h.query(|n| n.semantics.range == Some(range)).len(), 1);
}

fn numbers(stepper: bool) -> (Harness, Signal<f64>) {
    let count = signal(3.0f64);
    let h = mount(move || {
        let field = number_input(count).range(0.0, 5.0).test_id("count");
        column().child(if stepper { field.stepper() } else { field })
    });
    (h, count)
}

#[test]
fn a_number_input_holds_a_number_typed_or_stepped() {
    let (mut h, count) = numbers(false);
    assert!(h.semantics().contains("TextInput = \"3\" #count"), "{}", h.semantics());
    // The chevrons step it, and stop at the ends of its range.
    for _ in 0..4 {
        h.get_by_role(Role::Button, "Increment").click();
    }
    settle(&mut h);
    assert!(count.get_untracked() == 5.0 && h.semantics().contains("= \"5\""));
    // So do the up and down arrows while the field has the focus.
    h.get_by_test_id("count").focus();
    h.key(Key::Down, Mods::NONE);
    h.key(Key::Down, Mods::NONE);
    settle(&mut h);
    assert_eq!(count.get_untracked(), 3.0);
    // A number typed is the value, with a comma or a point; what is no number changes nothing.
    h.get_by_test_id("count").set_value("1,5");
    settle(&mut h);
    assert_eq!(count.get_untracked(), 1.5);
    h.get_by_test_id("count").set_value("many");
    settle(&mut h);
    assert_eq!(count.get_untracked(), 1.5);
    // A value written from elsewhere is what the field says.
    count.set(4.0);
    settle(&mut h);
    assert!(h.semantics().contains("TextInput = \"4\""), "{}", h.semantics());

    // The stepper is one frame of 128 by 32 with a button at each end.
    let (mut h, count) = numbers(true);
    let minus = h.get_by_role(Role::Button, "Decrement").node();
    let frame = h.tree.parent(minus).unwrap();
    assert_eq!(h.tree.rect(frame).size(), Size::new(128.0, 32.0));
    h.get_by_role(Role::Button, "Decrement").click();
    settle(&mut h);
    assert!(count.get_untracked() == 2.0 && h.semantics().contains("= \"2\""));
}

#[test]
fn a_field_says_what_is_wrong_in_place_of_its_help() {
    let (name, notes) = (signal(String::new()), signal("first".to_string()));
    let rules = [Rule::Required("A name is needed".into()), Rule::MinLen(3, "At least three letters".into())];
    let mut h = mount(move || {
        let wrong = move || validate(&name.get(), &rules);
        let named = field("Name", input(name).test_id("name")).help("As it shows to others").required();
        column().child(named.error(wrong)).child(field("Notes", textarea(notes).rows(3)))
    });
    let tree = h.semantics();
    assert!(tree.contains("Text \"Name\"") && tree.contains("Text \"*\""), "{tree}");
    assert!(h.has_text("A name is needed") && !h.has_text("As it shows to others"));
    h.get_by_test_id("name").set_value("Al");
    settle(&mut h);
    assert!(h.has_text("At least three letters"));
    h.get_by_test_id("name").set_value("Alan");
    settle(&mut h);
    assert!(h.has_text("As it shows to others") && !h.has_text("At least three letters"));
    // A text area of three lines is at least that tall, in the frame of an input.
    let area = h.get_by_text("first").node();
    let area = h.tree.parent(area).unwrap();
    assert!(h.tree.rect(area).height >= 3.0 * 23.0 + 14.0, "{:?}", h.tree.rect(area));

    let mail = Rule::Email("Not an address".into());
    let wrong = Some("Not an address");
    assert_eq!([mail.check("a@b.c"), mail.check("a b@c.d"), mail.check("@b.c")], [None, wrong, wrong]);
    assert_eq!(Rule::MaxLen(2, "Too long".into()).check("abc"), Some("Too long"));
}

#[test]
fn the_knobs_of_a_range_slider_move_to_the_pointer_and_never_pass_each_other() {
    use quadra_kit::range_slider;
    let (low, high) = (signal(20.0f32), signal(60.0f32));
    let mut h = mount(move || column().child(range_slider(low, high).label("Price")));
    let tree = h.semantics();
    for line in ["Group \"Price\"", "Slider \"Minimum\" = \"20\"", "Slider \"Maximum\" = \"60\""] {
        assert!(tree.contains(line), "{line} is not in:\n{tree}");
    }
    // A press moves the knob nearer to it; the track is 184 px from 8 px in.
    let at = |part: f32| 8.0 + 184.0 * part;
    h.click(at(0.3), 10.0);
    h.click(at(0.8), 10.0);
    assert_eq!((low.get_untracked(), high.get_untracked()), (30.0, 80.0));
    // A drag past the other knob goes on with that one: they never cross.
    h.pointer(PointerInput::Move, at(0.31), 10.0);
    h.pointer(PointerInput::Down, at(0.31), 10.0);
    h.pointer(PointerInput::Move, at(0.9), 10.0);
    h.pointer(PointerInput::Up, at(0.9), 10.0);
    assert_eq!((low.get_untracked(), high.get_untracked()), (31.0, 90.0));
    // Each knob has the arrows of its own, as far as the other lets it.
    h.get_by_role(Role::Slider, "Minimum").focus();
    h.key(Key::Right, Mods::NONE);
    h.key(Key::End, Mods::NONE);
    assert_eq!((low.get_untracked(), high.get_untracked()), (90.0, 90.0));
    h.get_by_role(Role::Slider, "Maximum").focus();
    h.key(Key::Home, Mods::NONE);
    h.key(Key::PageUp, Mods::NONE);
    settle(&mut h);
    assert_eq!((low.get_untracked(), high.get_untracked()), (90.0, 100.0));
    assert!(h.semantics().contains("Slider \"Maximum\" = \"100\""));
}

#[test]
fn a_combobox_offers_the_options_its_text_matches() {
    use quadra_kit::combobox;
    let (city, picked) = (signal(String::new()), signal(None));
    let mut h = mount(move || {
        let cities =
            ["H\u{E0} N\u{1ED9}i", "H\u{1EA3}i Ph\u{F2}ng", "Hu\u{1EBF}", "\u{110}\u{E0} N\u{1EB5}ng"];
        column().child(combobox(cities, city).placeholder("City").on_pick(move |i| picked.set(Some(i))))
    });
    let options = |h: &mut Harness| {
        let says = h.tree.semantic_tree();
        let rows = says.nodes.iter().filter(|n| n.semantics.role == Role::ListItem);
        rows.map(|n| format!("{}{}", n.semantics.name, if n.semantics.selected { "*" } else { "" }))
            .collect::<Vec<_>>()
    };
    assert!(options(&mut h).is_empty());
    // Typing opens the list on what matches, whatever the accents, the best first.
    h.get_by_role(Role::TextInput, "City").focus();
    h.type_text("ha");
    settle(&mut h);
    assert_eq!(options(&mut h), ["H\u{E0} N\u{1ED9}i*", "H\u{1EA3}i Ph\u{F2}ng"]);
    // It opens under the input, as wide as it, and the focus stays in the input.
    let (field, list) = (h.get_by_role(Role::TextInput, "City").node(), h.tree.overlays().next().unwrap());
    let (frame, list) = (h.tree.window_rect(h.tree.parent(field).unwrap()), h.tree.window_rect(list));
    assert_eq!(
        (list.x, list.y, list.width, h.tree.focused()),
        (frame.x, frame.bottom() + 4.0, 280.0, Some(field))
    );
    // The arrows go round the options; Enter takes the highlighted one and closes the list.
    h.key(Key::Down, Mods::NONE);
    settle(&mut h);
    assert_eq!(options(&mut h)[1], "H\u{1EA3}i Ph\u{F2}ng*");
    h.key(Key::Enter, Mods::NONE);
    settle(&mut h);
    assert_eq!((city.get_untracked().as_str(), picked.get_untracked()), ("H\u{1EA3}i Ph\u{F2}ng", Some(1)));
    assert!(options(&mut h).is_empty() && h.tree.focused() == Some(field));

    // Escape closes it; a click on an option takes it.
    h.get_by_role(Role::TextInput, "City").set_value("n");
    settle(&mut h);
    assert_eq!(options(&mut h).len(), 3);
    h.key(Key::Escape, Mods::NONE);
    settle(&mut h);
    assert!(options(&mut h).is_empty());
    h.key(Key::Down, Mods::NONE); // the arrow opens it again
    settle(&mut h);
    h.get_by_role(Role::ListItem, "\u{110}\u{E0} N\u{1EB5}ng").click();
    settle(&mut h);
    assert_eq!(
        (city.get_untracked().as_str(), picked.get_untracked()),
        ("\u{110}\u{E0} N\u{1EB5}ng", Some(3))
    );
    assert!(options(&mut h).is_empty());
}
