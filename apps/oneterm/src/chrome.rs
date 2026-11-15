//! The window's chrome (specs/19, "Window"): the title bar of 38 px with the app's mark and
//! what the active tab is, and the status bar of 26 px in mono 11 px with the working
//! directory (a click copies it) and the clock.

use std::rc::Rc;
use std::time::Duration;

use crate::model::Model;
use quadra::{
    Align, Clipboard, Display, Length, Signal, View, by_text, div, interval, row, text, time, use_context,
};
use quadra_design::{fonts, icon, icon_button, icons, titlebar, token, tooltip};

use crate::locked::shown;
use crate::signals::{all_bells, percent};

/// The terminal's mark in the accent, "1Term", then the active tab's title and the count of
/// its spaces in the secondary colour.
pub(crate) fn title_bar(model: &Model) -> impl View + use<> {
    let (model, seen, lock) = (model.clone(), model.clone(), model.lock());
    let said = move || {
        if lock.locked.get() {
            return "Locked".to_string();
        }
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
        .child(div().text_color(token::TEXT_2).child(text(said)));
    let open = seen.ui().settings;
    let gear = tooltip(
        "Settings",
        icon_button(icons::SETTINGS, "Settings").on_click(move || open.update(|on| *on = !*on)),
    );
    let locks = tooltip("Lock", icon_button(icons::LOCK, "Lock").on_click(move || lock.lock_now()));
    // Locked, what would open something is not displayed.
    let buttons = row()
        .align_items(Align::Center)
        .display(shown(move || !lock.locked.get()))
        .child(div().display(shown(move || lock.can_lock())).child(locks))
        .child(notifications(&seen))
        .child(gear);
    titlebar("").height(38.0).control(mark).control(div().grow(1.0)).control(buttons)
}

/// The button of notifications, with how many came unseen; a press marks them seen.
fn notifications(model: &crate::model::Model) -> impl View + use<> {
    let unread = model.unread();
    let count = div()
        .absolute()
        .top(2.0)
        .right(0.0)
        .padding_x(4.0)
        .radius(7.0)
        .bg(token::ACCENT)
        .text_color(token::TEXT_ON_ACCENT)
        .text_size(10.0)
        .display(move || if unread.get() > 0 { Display::Flex } else { Display::None })
        .child(text(move || unread.get().to_string()));
    let button = icon_button(icons::BELL, "Notifications").on_click(move || unread.set(0));
    div().child(tooltip("Notifications", button)).child(count)
}

/// The working directory at the left, the clock at the right.
pub(crate) fn status_bar(model: &Model, clock: Signal<String>) -> impl View + use<> {
    tick(clock);
    let (tabs, unread, shows, prefs) = (model.tabs(), model.unread(), model.clone(), model.prefs());
    // Locked, only the progress and the clock show.
    let lock = model.lock();
    let open = move || !lock.locked.get();
    let model = model.clone();
    let cwd = move || {
        let space = model.active_tab().and_then(|tab| tab.focused());
        space.map(|space| space.session.cwd().get()).unwrap_or_default()
    };
    let (clipboard, said, repo_of) = (use_context::<Rc<dyn Clipboard>>(), cwd.clone(), cwd.clone());
    let repo = crate::git::follow(repo_of);
    let git = div()
        .label("Git")
        .display(shown(move || open() && prefs.show_git.get() && repo.with(Option::is_some)))
        .child(text(move || match repo.get() {
            Some(repo) => match repo.changes {
                Some(n) if n > 0 => format!("{} \u{B7} {n} changed", repo.branch),
                _ => repo.branch,
            },
            None => String::new(),
        }));
    let place = row()
        .align_items(Align::Center)
        .gap(6.0)
        .label("Working directory")
        .display(shown(open))
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
        .child(host(&shows))
        .child(git)
        .child(div().grow(1.0))
        .child(meter(&shows))
        .child(
            div()
                .label("Usage")
                .display(shown(move || open() && prefs.show_usage.get()))
                .child(text(usage())),
        )
        .child(div().display(shown(move || !open())).child(text("Locked · details hidden")))
        .child(
            div()
                .label("Bells")
                .display(shown(open))
                .child(text(move || format!("bell {}", all_bells(tabs)))),
        )
        .child(
            div().label("Notices").display(shown(open)).child(text(move || format!("{} new", unread.get()))),
        )
        .child(
            div()
                .label("Clock")
                .display(shown(move || prefs.show_clock.get()))
                .child(text(move || clock.get())),
        )
}

/// The focused space's host and its latency, `prod-api-01 · 124 ms`, or `local`.
fn host(model: &Model) -> impl View + use<> {
    let model = model.clone();
    let lock = model.lock();
    let said = move || {
        let space = model.active_tab().and_then(|tab| tab.focused());
        match space.and_then(|space| space.remote.clone().map(|it| (it, space.session.link_note().get()))) {
            Some((remote, note)) if note.is_empty() => model.host_name(&remote),
            Some((remote, note)) => format!("{} \u{B7} {note}", model.host_name(&remote)),
            None => "local".into(),
        }
    };
    div().label("Host").display(shown(move || !lock.locked.get())).child(text(said))
}

/// The progress of the tab that shows: a meter of 56 px and its percentage.
fn meter(model: &crate::model::Model) -> impl View + use<> {
    let (model, shows) = (model.clone(), model.prefs().progress_bar);
    let part = quadra::memo(move || model.active_tab().and_then(|tab| tab.progress()).and_then(percent));
    let fill = div()
        .height(4.0)
        .radius(2.0)
        .bg(token::ACCENT)
        .width(move || Length::Pct(f32::from(part.get().unwrap_or(0))));
    let track = div().width(56.0).height(4.0).radius(2.0).bg(token::LINE_STRONG).child(fill);
    row()
        .align_items(Align::Center)
        .gap(6.0)
        .label("Progress")
        .display(move || if shows.get() && part.get().is_some() { Display::Flex } else { Display::None })
        .child(track)
        .child(text(move || part.get().map(|n| format!("{n}%")).unwrap_or_default()))
}

/// The processor and memory 1Term uses, read every 2 s: `CPU 3% · 182 MB`.
fn usage() -> Signal<String> {
    let said = quadra::signal(String::new());
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get()) as f64;
    let last = std::cell::Cell::new(None::<(std::time::Instant, Duration)>);
    let read = move || {
        #[cfg(windows)]
        if let Some(now) = quadra_shell::process_usage() {
            let at = std::time::Instant::now();
            let cpu = match last.replace(Some((at, now.cpu))) {
                Some((then, was)) => {
                    let wall = at.duration_since(then).as_secs_f64().max(1e-3);
                    (now.cpu.saturating_sub(was).as_secs_f64() / wall / cores * 100.0).round()
                }
                None => 0.0,
            };
            said.set(format!("CPU {cpu}% \u{B7} {} MB", now.memory / (1024 * 1024)));
        }
    };
    read();
    let _ = interval(Duration::from_secs(2), read);
    said
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
