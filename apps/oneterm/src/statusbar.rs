//! The status bar (specs/19, "Window"): 30 px on the chrome plane, 12 px, its groups parted by
//! rules of 1×14: the rail's fold; the focused space's directory (a click copies it), git, the
//! host and its latency; at the right the progress, 1Term's processor and memory, the bells
//! and notices, the lock's minutes, the clock and the SFTP panel. Locked, only a lock and
//! "Locked, details hidden", the progress and the clock show.

use std::rc::Rc;
use std::time::Duration;

use quadra::{
    Align, Clipboard, Length, Role, Signal, View, by_text, div, interval, row, text, time, use_context,
};
use quadra_design::{Variant, fonts, icon, icon_button, icons, token};

use crate::locked::shown;
use crate::model::Model;
use crate::signals::{all_bells, percent};

/// What shows when; a rule stands before a group that shows after one that shows.
type When = Rc<dyn Fn() -> bool>;

/// A part of a group: its icon or label and its value, 6 px apart.
fn part() -> quadra::Div {
    row().align_items(Align::Center).gap(6.0)
}

/// A value: mono, in the text colour.
fn value(words: impl Fn() -> String + 'static) -> quadra::Div {
    div().font_family(fonts::MONO).text_color(token::TEXT).child(text(words))
}

/// Groups in a row, a rule before each that shows after one that showed.
fn groups(row_of: quadra::Div, all: Vec<(When, quadra::Div)>) -> quadra::Div {
    let whens: Vec<When> = all.iter().map(|(it, _)| it.clone()).collect();
    let mut line = row_of;
    for (i, (when, group)) in all.into_iter().enumerate() {
        let (before, me, ruled) = (whens[..i].to_vec(), when.clone(), when);
        let rule = div().width(1.0).height(14.0).shrink(0.0).bg(token::LINE_STRONG);
        line = line.child(rule.display(shown(move || ruled() && before.iter().any(|it| it()))));
        line = line.child(group.display(shown(move || me())));
    }
    line
}

fn when(f: impl Fn() -> bool + 'static) -> When {
    Rc::new(f)
}

pub(crate) fn status_bar(model: &Model, clock: Signal<String>) -> impl View + use<> {
    tick(clock);
    let (tabs, unread, prefs, lock) = (model.tabs(), model.unread(), model.prefs(), model.lock());
    let open = move || !lock.locked.get();
    let (vertical, folded) = (model.vertical(), prefs.rail_folded);
    let fold = icon_button(icons::PANEL_LEFT, "Collapse tabs to icons")
        .variant(Variant::Link)
        .label(move || if folded.get() { "Expand tabs" } else { "Collapse tabs to icons" }.to_string())
        .on_click(move || folded.update(|on| *on = !*on));
    let cwd_of = model.clone();
    let cwd = move || {
        let space = cwd_of.active_tab().and_then(|tab| tab.focused());
        space.map(|space| space.session.cwd().get()).unwrap_or_default()
    };
    let (clipboard, said, repo_of) = (use_context::<Rc<dyn Clipboard>>(), cwd.clone(), cwd.clone());
    let repo = crate::git::follow(repo_of);
    let place = part()
        .role(Role::Button)
        .label("Working directory")
        .on_click(move |_| {
            if let Some(clipboard) = &clipboard {
                clipboard.write_text(&quadra::untrack(&cwd));
            }
        })
        .child(icon(icons::FOLDER).size(14.0))
        .child(value(said));
    let counts = move || repo.get().and_then(|it| it.changes).unwrap_or_default();
    let count = |color, sign: &'static str, n: fn((usize, usize)) -> usize| {
        div()
            .font_family(fonts::MONO)
            .text_color(color)
            .display(shown(move || n(counts()) > 0))
            .child(text(move || format!("{sign}{}", n(counts()))))
    };
    let git = part()
        .label("Git")
        .display(shown(move || prefs.show_git.get() && repo.with(Option::is_some)))
        .child(icon(icons::GIT_BRANCH).size(14.0))
        .child(value(move || repo.get().map(|it| it.branch).unwrap_or_default()))
        .child(count(token::HUE_GREEN, "\u{2191}", |it| it.0))
        .child(count(token::HUE_YELLOW, "M", |it| it.1));
    let usage = usage();
    let using = part()
        .gap(12.0)
        .label("Usage")
        .child(
            part()
                .display(shown(move || prefs.show_usage.get()))
                .child(text("CPU"))
                .child(value(move || usage.get().0)),
        )
        .child(
            part()
                .display(shown(move || prefs.show_usage.get()))
                .child(text("RAM"))
                .child(value(move || usage.get().1)),
        )
        .child(part().display(shown(move || prefs.show_fps.get())).child({
            let said = fps(prefs.show_fps);
            value(move || said.get())
        }));
    // Bells not heard and notices not seen, one count; a click marks the notices seen.
    let count = move || all_bells(tabs) + unread.get();
    let tone = move || if count() > 0 { token::ERROR } else { token::TEXT_2 };
    let bells = part()
        .padding_x(4.0)
        .height(22.0)
        .radius(4.0)
        .hover(|s| s.bg(token::PANEL_HOVER))
        .role(Role::Button)
        .label(move || format!("{} bells and notices", count()))
        .display(shown(open))
        .on_click(move |_| unread.set(0))
        .child(div().text_color(tone).child(icon(icons::BELL).size(14.0)))
        .child(div().font_family(fonts::MONO).text_color(tone).child(text(move || count().to_string())));
    let left = move || lock.left();
    let locks = part()
        .label("Lock")
        .display(shown(move || open() && left().is_some()))
        .child(icon(icons::LOCK).size(14.0))
        .child(div().font_family(fonts::MONO).child(text(move || format!("{}m", left().unwrap_or(0)))));
    let time = value(move || clock.get()).label("Clock").display(shown(move || prefs.show_clock.get()));
    let hidden = part()
        .display(shown(move || !open()))
        .child(icon(icons::LOCK).size(14.0))
        .child(text("Locked, details hidden"));
    let shows = move || prefs.show_usage.get() || prefs.show_fps.get();
    let (meter, metered) = meter(model);
    let left_side = groups(
        part().gap(12.0),
        vec![
            (when(move || open() && vertical.get()), part().child(fold)),
            (when(open), part().gap(12.0).child(place).child(git)),
            (when(open), host(model)),
        ],
    );
    let right_side = groups(
        part().gap(12.0),
        vec![
            (metered, meter),
            (when(move || open() && shows()), using),
            (
                when(move || open() || prefs.show_clock.get()),
                part().gap(12.0).child(bells).child(locks).child(time),
            ),
            (when(open), part().child(sftp(model))),
        ],
    );
    row()
        .align_items(Align::Center)
        .gap(12.0)
        .padding_x(8.0)
        .height(by_text(30.0))
        .shrink(0.0)
        .bg(token::STATUS_BG)
        .border_top(1.0)
        .border_color(token::LINE)
        .text_size(12.0)
        .text_color(token::TEXT_2)
        .test_id("status")
        .child(hidden)
        .child(left_side)
        .child(div().grow(1.0))
        .child(right_side)
}

/// Toggle SFTP panel, in the accent while the dock shows; zoomed in, it zooms out.
fn sftp(model: &Model) -> impl View + use<> {
    let (dock, expanded) = (model.ui().dock, model.ui().expanded);
    icon_button(icons::PANEL_RIGHT, "Toggle SFTP panel")
        .label(move || {
            if expanded.get() { "SFTP is zoomed in; zoom out" } else { "Toggle SFTP panel" }.to_string()
        })
        .toggled(move || dock.get())
        .on_click(move || match expanded.get_untracked() {
            true => expanded.set(false),
            false => dock.update(|on| *on = !*on),
        })
}

/// The focused space's host and its latency, `prod-api-01 124 ms`, or `local shell`.
fn host(model: &Model) -> quadra::Div {
    let (model, of) = (model.clone(), model.clone());
    let remote = move || of.active_tab().and_then(|tab| tab.focused()).and_then(|space| space.remote);
    let (link, is_remote, is_local) = (model.clone(), remote.clone(), remote.clone());
    let name = move || match remote() {
        Some(remote) => model.host_name(&remote),
        None => "local shell".into(),
    };
    let note = move || {
        let space = link.active_tab().and_then(|tab| tab.focused());
        space.filter(|it| it.remote.is_some()).map(|it| it.session.link_note().get()).unwrap_or_default()
    };
    part()
        .label("Host")
        .child(div().display(shown(move || is_remote().is_some())).child(icon(icons::SERVER).size(14.0)))
        .child(
            div().display(shown(move || is_local().is_none())).child(icon(icons::SQUARE_TERMINAL).size(14.0)),
        )
        .child(text(name))
        .child(div().font_family(fonts::MONO).child(text(note)))
}

/// The progress of the tab that shows, else of the first that reports one: the id of what
/// reports it (`build`), a line of 56 px and the percentage; and whether it shows.
fn meter(model: &Model) -> (quadra::Div, When) {
    let (model, shows) = (model.clone(), model.prefs().progress_bar);
    // The first space that reports a part done, and the id of what it runs (OSC 7501).
    let found = quadra::memo(move || {
        let tabs = model.tabs().get();
        let mut first = model.active_tab().into_iter().chain(tabs);
        first.find_map(|tab| {
            tab.spaces.with(|all| {
                all.iter().find_map(|it| {
                    let n = it.session.progress().get().and_then(percent)?;
                    Some((n, it.session.status_name().get()))
                })
            })
        })
    });
    let fill = div()
        .height(2.0)
        .bg(token::ACCENT)
        .width(move || Length::Pct(f32::from(found.with(|it| it.as_ref().map_or(0, |it| it.0)))));
    let name = move || found.with(|it| it.as_ref().map(|it| it.1.clone()).unwrap_or_default());
    let meter = part()
        .label("Progress")
        .child(div().max_width(Length::Px(120.0)).child(text(name).ellipsis()))
        .child(div().width(56.0).height(2.0).bg(token::LINE_STRONG).child(fill))
        .child(value(move || {
            found.with(|it| it.as_ref().map(|it| format!("{}%", it.0)).unwrap_or_default())
        }));
    (meter, when(move || shows.get() && found.with(Option::is_some)))
}

/// The frames the window made in the last second, `60 fps`, counted while shown.
fn fps(shown: Signal<bool>) -> Signal<String> {
    let (said, window) = (quadra::signal(String::new()), quadra::use_context::<quadra::Window>());
    let last = std::cell::Cell::new(0u64);
    let _ = interval(Duration::from_secs(1), move || {
        let made = window.as_ref().map_or(0, quadra::Window::frames);
        let n = made.saturating_sub(last.replace(made));
        if shown.get_untracked() {
            said.set(format!("{n} fps"));
        }
    });
    said
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

/// Keeps `clock` the local time, `16:09`.
fn tick(clock: Signal<String>) {
    let now = move || {
        let at = time::local_now();
        format!("{:02}:{:02}", at.time.hour, at.time.minute)
    };
    clock.set(now());
    // A second at most late: the minute shows when it changes.
    let _ = interval(Duration::from_secs(1), move || clock.set(now()));
}
