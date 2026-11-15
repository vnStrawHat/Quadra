//! The status bar (specs/19, "Window"): 26 px in mono 11 px, its parts parted by rules of
//! 14 px: the working directory (a click copies it), git, the host and its latency; at the
//! right the progress, 1Term's processor and memory, the bells and notices, the lock's time
//! and the clock. Locked, only the progress and the clock show.

use std::rc::Rc;
use std::time::Duration;

use quadra::{
    Align, Clipboard, Display, Length, Role, Signal, View, by_text, div, interval, row, text, time,
    use_context,
};
use quadra_design::{fonts, icon, icons, token};

use crate::locked::shown;
use crate::model::Model;
use crate::signals::{all_bells, percent};

/// A part of the bar: what it holds 8 px in from its sides.
fn part() -> quadra::Div {
    row().align_items(Align::Center).gap(5.0).padding_x(8.0)
}

/// A rule before a part, shown with it.
fn ruled(when: impl Fn() -> bool + 'static, part: impl View + 'static) -> impl View {
    row()
        .align_items(Align::Center)
        .display(shown(when))
        .child(div().width(1.0).height(14.0).bg(token::LINE))
        .child(part)
}

fn bright(words: impl Fn() -> String + 'static) -> quadra::Div {
    div().text_color(token::TEXT).child(text(words))
}

pub(crate) fn status_bar(model: &Model, clock: Signal<String>) -> impl View + use<> {
    tick(clock);
    let (tabs, unread, shows, prefs) = (model.tabs(), model.unread(), model.clone(), model.prefs());
    let lock = model.lock();
    let open = move || !lock.locked.get();
    let model = model.clone();
    let cwd = move || {
        let space = model.active_tab().and_then(|tab| tab.focused());
        space.map(|space| space.session.cwd().get()).unwrap_or_default()
    };
    let (clipboard, said, repo_of) = (use_context::<Rc<dyn Clipboard>>(), cwd.clone(), cwd.clone());
    let repo = crate::git::follow(repo_of);
    let place = part()
        .label("Working directory")
        .display(shown(open))
        .on_click(move |_| {
            if let Some(clipboard) = &clipboard {
                clipboard.write_text(&quadra::untrack(&cwd));
            }
        })
        .child(icon(icons::FOLDER).size(12.0).color(token::TEXT_2))
        .child(bright(said));
    let changes = move || repo.get().and_then(|it| it.changes).filter(|n| *n > 0);
    let git = part()
        .label("Git")
        .child(icon(icons::WORKFLOW).size(12.0).color(token::TEXT_2))
        .child(bright(move || repo.get().map(|it| it.branch).unwrap_or_default()))
        .child(
            div()
                .text_color(token::WARN)
                .display(shown(move || changes().is_some()))
                .child(text(move || format!("~{}", changes().unwrap_or(0)))),
        );
    let usage = usage();
    let (cpu, memory) = (move || usage.get().0, move || usage.get().1);
    let using =
        part().label("Usage").child(text("CPU")).child(bright(cpu)).child(text("RAM")).child(bright(memory));
    // Bells not heard and notices not seen, one count; a click marks the notices seen.
    let count = move || all_bells(tabs) + unread.get();
    let bells = part()
        .role(Role::Button)
        .label(move || format!("{} bells and notices", count()))
        .on_click(move |_| unread.set(0))
        .child(icon(icons::BELL).size(12.0).color(token::TEXT_2))
        .child(bright(move || count().to_string()));
    let left = move || lock.left();
    let locks = part()
        .label("Lock")
        .child(icon(icons::LOCK).size(12.0).color(token::TEXT_2))
        .child(text(move || format!("lock in {}m", left().unwrap_or(0))));
    let hidden = part()
        .display(shown(move || !open()))
        .child(icon(icons::LOCK).size(12.0).color(token::TEXT_2))
        .child(text("Locked \u{B7} details hidden"));
    row()
        .align_items(Align::Center)
        .gap(2.0)
        .padding_x(8.0)
        .height(by_text(26.0))
        .shrink(0.0)
        .bg(token::STATUS_BG)
        .border_top(1.0)
        .border_color(token::LINE)
        .font_family(fonts::MONO)
        .text_size(11.0)
        .text_color(token::TEXT_2)
        .test_id("status")
        .child(place)
        .child(hidden)
        .child(ruled(move || open() && prefs.show_git.get() && repo.with(Option::is_some), git))
        .child(ruled(open, host(&shows)))
        .child(div().grow(1.0))
        .child(meter(&shows))
        .child(ruled(move || open() && prefs.show_usage.get(), using))
        .child(ruled(open, bells))
        .child(ruled(move || open() && left().is_some(), locks))
        .child(
            part()
                .label("Clock")
                .display(shown(move || prefs.show_clock.get()))
                .child(bright(move || clock.get())),
        )
}

/// The focused space's host and its latency, `● prod-api-01 · 124 ms`, or `local`.
fn host(model: &Model) -> impl View + use<> {
    let model = model.clone();
    let remote = {
        let model = model.clone();
        move || model.active_tab().and_then(|tab| tab.focused()).and_then(|space| space.remote).is_some()
    };
    let said = move || {
        let space = model.active_tab().and_then(|tab| tab.focused());
        match space.and_then(|space| space.remote.clone().map(|it| (it, space.session.link_note().get()))) {
            Some((remote, note)) if note.is_empty() => model.host_name(&remote),
            Some((remote, note)) => format!("{} \u{B7} {note}", model.host_name(&remote)),
            None => "local".into(),
        }
    };
    let dot = div().size(6.0, 6.0).radius(3.0).bg(token::OK).display(shown(remote));
    part().label("Host").child(dot).child(text(said))
}

/// The progress of the tab that shows: a meter of 56 px and its percentage.
fn meter(model: &Model) -> impl View + use<> {
    let (model, shows) = (model.clone(), model.prefs().progress_bar);
    let part_of = quadra::memo(move || model.active_tab().and_then(|tab| tab.progress()).and_then(percent));
    let fill = div()
        .height(4.0)
        .radius(2.0)
        .bg(token::ACCENT)
        .width(move || Length::Pct(f32::from(part_of.get().unwrap_or(0))));
    let track = div().width(56.0).height(4.0).radius(2.0).bg(token::LINE_STRONG).child(fill);
    part()
        .gap(6.0)
        .label("Progress")
        .display(move || if shows.get() && part_of.get().is_some() { Display::Flex } else { Display::None })
        .child(track)
        .child(
            div()
                .text_color(token::ACCENT)
                .child(text(move || part_of.get().map(|n| format!("{n}%")).unwrap_or_default())),
        )
}

/// The processor and memory 1Term uses, read every 2 s: `3%`, `182 MB`.
fn usage() -> Signal<(String, String)> {
    let said = quadra::signal((String::new(), String::new()));
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
            said.set((format!("{cpu}%"), format!("{} MB", now.memory / (1024 * 1024))));
        }
    };
    read();
    let _ = interval(Duration::from_secs(2), read);
    said
}

/// Keeps `clock` the local time: `We 28 Oct · 11:00`, the day and month in the app's language.
fn tick(clock: Signal<String>) {
    let now = move || {
        let (locale, at) = (quadra::language().locale(), time::local_now());
        let short: String = locale.month(at.date.month).chars().take(3).collect();
        let day = locale.day(at.date.weekday());
        format!("{day} {} {short} \u{B7} {:02}:{:02}", at.date.day, at.time.hour, at.time.minute)
    };
    clock.set(now());
    // A second at most late: the minute shows when it changes.
    let _ = interval(Duration::from_secs(1), move || clock.set(now()));
}
