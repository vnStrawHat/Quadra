//! The headless crate's values with bare parts (specs/17, "Tests"): a slider and a range, a number
//! input, a time, a day, rules.

use std::rc::Rc;

use quadra_headless::rules::{Rule, validate};
use quadra_headless::slider::{self, Scale};
use quadra_headless::{number, time};
use quadra_platform::{Key, Mods};
use quadra_reactive::{memo, signal};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role, SemanticAction};
use quadra_view::{View, column, div, text, text_input};

fn mounted<V: View + 'static>(view: impl Fn() -> V + 'static) -> Harness {
    let mut h = Harness::new();
    h.tree.set_keymap(Rc::new(quadra_headless::keymap()));
    h.mount(view);
    h.frame();
    h
}

#[test]
fn a_slider_takes_a_press_a_drag_the_keys_and_a_reader_and_a_range_keeps_its_order() {
    let (value, low, high) = (signal(50.0f32), signal(20.0f32), signal(60.0f32));
    let scale = Scale::new((0.0, 100.0), 5.0, 0.0, 100.0);
    let mut h = mounted(move || {
        let one = slider::slider(div().size(100.0, 10.0).label("Volume"), value, scale, memo(|| false));
        let pair = slider::pair(div().size(100.0, 10.0).label("Price"), low, high, scale);
        let pair = pair.child(slider::end(div(), low, high, scale, true)).child(slider::end(
            div(),
            low,
            high,
            scale,
            false,
        ));
        column().child(one).child(pair)
    });
    assert!(h.semantics().contains("Slider \"Volume\""), "{}", h.semantics());
    let face = h.get_by_role(Role::Slider, "Volume").node();
    let at = h.tree.window_rect(face);
    h.pointer(PointerInput::Move, at.x + 23.0, at.y + 5.0);
    h.pointer(PointerInput::Down, at.x + 23.0, at.y + 5.0);
    assert_eq!(value.get_untracked(), 25.0); // to the step
    h.pointer(PointerInput::Move, at.x + 200.0, at.y + 5.0);
    h.pointer(PointerInput::Up, at.x + 200.0, at.y + 5.0);
    assert_eq!(value.get_untracked(), 100.0);
    h.tree.focus(face);
    h.key(Key::Left, Mods::NONE);
    h.key(Key::PageDown, Mods::NONE);
    assert_eq!(value.get_untracked(), 85.0);
    h.get_by_role(Role::Slider, "Volume").act(SemanticAction::SetValue("40".into()));
    assert_eq!(value.get_untracked(), 40.0);
    // The low end goes no further than the high one.
    let minimum = h.get_by_role(Role::Slider, "Minimum").node();
    h.tree.focus(minimum);
    h.key(Key::End, Mods::NONE);
    assert_eq!((low.get_untracked(), high.get_untracked()), (60.0, 60.0));
    // A press nearer the high end moves it.
    let pair = h.get_by_role(Role::Group, "Price").node();
    let at = h.tree.window_rect(pair);
    h.pointer(PointerInput::Move, at.x + 90.0, at.y + 5.0);
    h.pointer(PointerInput::Down, at.x + 90.0, at.y + 5.0);
    h.pointer(PointerInput::Up, at.x + 90.0, at.y + 5.0);
    assert_eq!(high.get_untracked(), 90.0);
}

#[test]
fn a_number_follows_its_text_and_steps_within_its_ends() {
    let value = signal(5.0f64);
    let mut h = mounted(move || {
        let n = number::number(value, (0.0, 6.0), 1.0, 1);
        let field = text_input(n.words).test_id("n");
        n.keys(div()).child(field).child(n.stepper(div().size(10.0, 10.0), true))
    });
    h.get_by_role(Role::Button, "Increment").click();
    h.get_by_role(Role::Button, "Increment").click();
    h.frame();
    assert_eq!(value.get_untracked(), 6.0);
    assert!(h.semantics().contains("\"6.0\""), "{}", h.semantics());
    let field = h.get_by_test_id("n").node();
    h.tree.focus(field);
    h.key(Key::Down, Mods::NONE);
    assert_eq!(value.get_untracked(), 5.0);
    h.type_text("x");
    assert!(value.get_untracked() == 5.0, "what is no number changes nothing");
    assert_eq!(number::parse(" 2,5 "), Some(2.5));
    assert_eq!(number::parse("two"), None);
}

#[test]
fn a_part_of_a_time_steps_round_and_is_typed_and_a_day_is_chosen() {
    let (minute, day) = (signal(58u8), signal(1u32));
    let mut h = mounted(move || {
        let part = time::part(
            div().child(text("m")),
            "Minutes",
            (60, 1),
            move || minute.get(),
            move |m| minute.set(m),
        );
        let days = (1..=3).map(|d| time::day(div().size(10.0, 10.0), format!("Day {d}"), d, day, || {}));
        column().child(part).children(days.collect::<Vec<_>>())
    });
    let part = h.get_by_role(Role::Slider, "Minutes").node();
    h.tree.focus(part);
    h.key(Key::Up, Mods::NONE);
    h.key(Key::Up, Mods::NONE);
    assert_eq!(minute.get_untracked(), 0); // round its end
    h.type_text("4");
    h.type_text("5");
    assert_eq!(minute.get_untracked(), 45);
    assert_eq!(time::typed(4, 7, 24), 7, "47 is no hour");
    h.get_by_role(Role::Button, "Day 3").click();
    h.frame();
    assert_eq!(day.get_untracked(), 3);
    assert!(h.semantics().contains("Button \"Day 3\" [selected]"), "{}", h.semantics());
    assert_eq!(time::TimeOfDay::new(9, 5).to_string(), "09:05");
}

#[test]
fn the_first_broken_rule_is_what_a_field_says() {
    let rules = [Rule::Required("Needed".into()), Rule::Email("Not an address".into())];
    assert_eq!(validate("", &rules), "Needed");
    assert_eq!(validate("a@b", &rules), "Not an address");
    assert_eq!(validate("a@b.vn", &rules), "");
    assert_eq!(Rule::MinLen(3, "Short".into()).check("ab"), Some("Short"));
}
