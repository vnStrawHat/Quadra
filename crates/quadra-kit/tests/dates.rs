//! Dates and times (specs/behaviour/09, "Forms"): the month calendar, the date picker that
//! opens one, and the time input of two parts.

use std::rc::Rc;

use quadra_kit::{Date, TimeOfDay, calendar, dark, date_picker, time_input};
use quadra_platform::{Key, Mods};
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::Role;
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
fn a_month_has_its_days_and_a_day_its_place_in_the_week() {
    assert_eq!([Date::days_in(2024, 2), Date::days_in(2023, 2), Date::days_in(1900, 2)], [29, 28, 28]);
    assert_eq!([Date::days_in(2000, 2), Date::days_in(2026, 11), Date::days_in(2026, 12)], [29, 30, 31]);
    // Monday is 0: a Wednesday, a Thursday, a Saturday, a Sunday.
    let days =
        [Date::new(2026, 11, 11), Date::new(2024, 2, 29), Date::new(2000, 1, 1), Date::new(2026, 11, 1)];
    assert_eq!(days.map(Date::weekday), [2, 3, 5, 6]);
    assert_eq!(Date::new(2026, 3, 5).to_string(), "2026-03-05"); // the form of files; a user is shown theirs
}

#[test]
fn a_calendar_shows_a_month_and_a_click_chooses_a_day() {
    let day = signal(Date::new(2026, 11, 11));
    let mut h = mount(move || column().child(calendar(day)));
    assert!(h.has_text("November 2026") && h.has_text("Mo") && h.has_text("Su"));
    let tree = h.semantics();
    assert!(
        tree.contains("Button \"11/11/2026\" [selected]") && tree.contains("Button \"30/11/2026\""),
        "{tree}"
    );
    // The first of November 2026 is a Sunday: the last of the seven places of its week.
    let (first, second) =
        (h.get_by_role(Role::Button, "01/11/2026").node(), h.get_by_role(Role::Button, "02/11/2026").node());
    let (first, second) = (h.tree.window_rect(first), h.tree.window_rect(second));
    assert_eq!(
        (first.width, first.height, first.x - second.x, second.y - first.y),
        (32.0, 28.0, 6.0 * 34.0, 30.0)
    );

    h.get_by_role(Role::Button, "15/11/2026").click();
    settle(&mut h);
    assert_eq!(day.get_untracked(), Date::new(2026, 11, 15));
    assert!(h.semantics().contains("Button \"15/11/2026\" [selected]"));
    // The arrows show the months around, across the end of a year; the day stays chosen.
    h.get_by_role(Role::Button, "Previous month").click();
    settle(&mut h);
    assert!(h.has_text("October 2026") && h.semantics().contains("Button \"31/10/2026\""));
    for _ in 0..3 {
        h.get_by_role(Role::Button, "Next month").click();
    }
    settle(&mut h);
    assert!(h.has_text("January 2027") && day.get_untracked() == Date::new(2026, 11, 15));
}

#[test]
fn a_date_picker_opens_a_calendar_and_a_day_closes_it() {
    let day = signal(Date::new(2026, 11, 11));
    let mut h = mount(move || column().child(date_picker(day)));
    assert!(h.semantics().contains("Button \"11/11/2026\" [collapsed]"), "{}", h.semantics());
    h.get_by_role(Role::Button, "11/11/2026").focus();
    h.key(Key::Enter, Mods::NONE);
    settle(&mut h);
    assert!(h.has_text("November 2026") && h.tree.overlays().count() == 1);
    h.get_by_role(Role::Button, "20/11/2026").click();
    settle(&mut h);
    assert_eq!((day.get_untracked(), h.tree.overlays().count()), (Date::new(2026, 11, 20), 0));
    assert!(h.semantics().contains("Button \"20/11/2026\" [collapsed]"));
}

#[test]
fn the_parts_of_a_time_are_stepped_and_typed() {
    let time = signal(TimeOfDay::new(9, 30));
    let mut h = mount(move || column().child(time_input(time).minute_step(15)));
    let tree = h.semantics();
    for line in ["Group \"09:30\"", "Slider \"Hours\" = \"09\"", "Slider \"Minutes\" = \"30\""] {
        assert!(tree.contains(line), "{line} is not in:\n{tree}");
    }
    // The arrows step the part with the focus, round its ends.
    h.get_by_role(Role::Slider, "Hours").focus();
    h.key(Key::Up, Mods::NONE);
    assert_eq!(time.get_untracked(), TimeOfDay::new(10, 30));
    h.key(Key::Home, Mods::NONE);
    h.key(Key::Down, Mods::NONE);
    assert_eq!(time.get_untracked(), TimeOfDay::new(23, 30));
    // Digits type it: two in a row make a number of two, a third starts again.
    h.type_text("1");
    h.type_text("5");
    assert_eq!(time.get_untracked().hour, 15);
    h.type_text("7");
    assert_eq!(time.get_untracked().hour, 7);
    h.type_text("9"); // 79 is no hour
    assert_eq!(time.get_untracked().hour, 9);
    // Tab goes on to the minutes, which step by what they were told.
    h.key(Key::Tab, Mods::NONE);
    h.key(Key::Up, Mods::NONE);
    h.key(Key::Up, Mods::NONE);
    assert_eq!(time.get_untracked(), TimeOfDay::new(9, 0));
    h.key(Key::End, Mods::NONE);
    settle(&mut h);
    assert!(time.get_untracked() == TimeOfDay::new(9, 59) && h.semantics().contains("Group \"09:59\""));
}
