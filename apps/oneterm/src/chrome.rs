//! The window's chrome (specs/19, "Window"): the title bar of 38 px with the app's mark and
//! what the active tab is, and the status bar of 26 px in mono 11 px with the working
//! directory (a click copies it) and the clock.

use std::rc::Rc;
use std::time::Duration;

use crate::model::Model;
use quadra::{Align, Clipboard, Signal, View, by_text, div, interval, row, text, time, use_context};
use quadra_design::{fonts, icon, icons, titlebar, token};

/// The terminal's mark in the accent, "1Term", then the active tab's title and the count of
/// its spaces in the secondary colour.
pub(crate) fn title_bar(model: &Model) -> impl View + use<> {
    let model = model.clone();
    let shown = move || {
        let Some(tab) = model.active_tab() else { return String::new() };
        let n = tab.spaces.with(Vec::len);
        format!("{} \u{B7} {n} {}", tab.title(), if n == 1 { "space" } else { "spaces" })
    };
    let mark = row()
        .align_items(Align::Center)
        .gap(8.0)
        .padding_left(12.0)
        .child(icon(icons::TERMINAL).size(16.0).color(token::ACCENT))
        .child(div().font_weight(600).child(text("1Term")))
        .child(div().text_color(token::TEXT_2).child(text(shown)));
    titlebar("").height(38.0).control(mark).control(div().grow(1.0))
}

/// The working directory at the left, the clock at the right.
pub(crate) fn status_bar(model: &Model, clock: Signal<String>) -> impl View + use<> {
    tick(clock);
    let model = model.clone();
    let cwd = move || {
        let space = model.active_tab().and_then(|tab| tab.focused());
        space.map(|space| space.session.cwd().get()).unwrap_or_default()
    };
    let (clipboard, said) = (use_context::<Rc<dyn Clipboard>>(), cwd.clone());
    let place = row()
        .align_items(Align::Center)
        .gap(6.0)
        .label("Working directory")
        .on_click(move |_| {
            if let Some(clipboard) = &clipboard {
                clipboard.write_text(&quadra::untrack(&cwd));
            }
        })
        .child(icon(icons::FOLDER).size(12.0).color(token::TEXT_2))
        .child(text(said));
    row()
        .align_items(Align::Center)
        .gap(12.0)
        .padding_x(10.0)
        .height(by_text(26.0))
        .shrink(0.0)
        .bg(token::STATUS_BG)
        .border_top(1.0)
        .border_color(token::LINE_STRONG)
        .font_family(fonts::MONO)
        .text_size(11.0)
        .text_color(token::TEXT_2)
        .test_id("status")
        .child(place)
        .child(div().grow(1.0))
        .child(div().label("Clock").child(text(move || clock.get())))
}

/// Keeps `clock` the local time as the app's locale says it: `We 28 Oct · 11:00`.
fn tick(clock: Signal<String>) {
    let now = move || {
        let (locale, at) = (quadra::language().locale(), time::local_now());
        let month = locale.month(at.date.month);
        let short: String = month.chars().take(3).collect();
        let day = locale.day(at.date.weekday());
        format!("{day} {} {short} \u{B7} {}", at.date.day, locale.time(at.time))
    };
    clock.set(now());
    // A second at most late: the minute shows when it changes.
    let _ = interval(Duration::from_secs(1), move || clock.set(now()));
}
