//! How signals look (specs/19, "Signals"): a tab's signal at the end of its row, its icon, the
//! seam bar of its progress, the second line of its row, a space header's signal.

use quadra::{Align, AnyView, Display, Length, Role, View, div, dynamic, row, text};
use quadra_design::{fonts, icon, icons, token};
use quadra_term::ProgramState;
use quadra_term::vt::Progress;

use crate::model::Tab;
use crate::signals::{percent, state_word};

/// A state's icon, outlined in its colour, and what a reader hears of it; none for idle.
fn state_look(state: ProgramState) -> Option<(&'static str, quadra::Token, &'static str)> {
    match state {
        ProgramState::Blocked => Some((icons::CIRCLE_HELP, token::WARN, "Blocked, needs permission")),
        ProgramState::Done => Some((icons::CIRCLE_CHECK, token::OK, "Done, result not yet seen")),
        ProgramState::Error => Some((icons::CIRCLE_X, token::ERROR, "Error")),
        ProgramState::Working => Some((icons::LOADER_CIRCLE, token::ACCENT, "Working")),
        ProgramState::Idle => None,
    }
}

/// The state's icon of `px`, named for a reader.
pub(crate) fn state_icon(state: ProgramState, px: f32) -> Option<quadra::Div> {
    let (body, color, said) = state_look(state)?;
    Some(div().shrink(0.0).role(Role::Image).label(said).child(icon(body).size(px).color(color)))
}

/// The bell that rang unseen, in the error colour.
fn bell(px: f32) -> quadra::Div {
    div()
        .shrink(0.0)
        .role(Role::Image)
        .label("Bell rang")
        .child(icon(icons::BELL).size(px).color(token::ERROR))
}

/// The part done of a tab's progress, when the settings show it on tabs.
fn part_of(tab: Tab) -> Option<u8> {
    tab.progress().filter(|_| tab.prefs.progress_tab.get()).and_then(percent)
}

/// What a tab says at its end: blocked, error and done as their icons, else its progress in
/// the accent, else working's spinner, else a bell when one rang unseen, else `idle`.
pub(crate) fn tab_mark(tab: Tab) -> impl View {
    dynamic(move || {
        let state = tab.status();
        let shown = !matches!(state, ProgramState::Working | ProgramState::Idle);
        match state_icon(state, 14.0).filter(|_| shown) {
            Some(face) => AnyView::new(face),
            None if part_of(tab).is_some() => AnyView::new(
                div()
                    .font_family(fonts::MONO)
                    .text_size(12.0)
                    .text_color(token::ACCENT)
                    .child(text(move || part_of(tab).map(|n| format!("{n}%")).unwrap_or_default())),
            ),
            None => match state_icon(state, 14.0) {
                Some(face) => AnyView::new(face),
                None if tab.bells() > 0 => AnyView::new(bell(14.0)),
                None => AnyView::new(div().text_size(12.0).text_color(token::TEXT_3).child(text("idle"))),
            },
        }
    })
}

/// The badge of a tab's icon in the folded rail: its state's icon or a bell, at the box's top
/// right; a progress is the seam bar, idle nothing.
pub(crate) fn tab_badge(tab: Tab) -> impl View {
    let face = dynamic(move || match state_icon(tab.status(), 12.0) {
        Some(_) if tab.status() == ProgramState::Working && part_of(tab).is_some() => AnyView::new(div()),
        Some(face) => AnyView::new(face),
        None if tab.bells() > 0 => AnyView::new(bell(12.0)),
        None => AnyView::new(div()),
    });
    div().absolute().top(5.0).right(7.0).child(face)
}

/// What a reader hears of a tab in the folded rail: its title, then its signal.
pub(crate) fn tab_said(tab: Tab) -> String {
    let state = tab.status();
    let signal = match state_look(state) {
        Some((.., said)) if state != ProgramState::Working => said.to_lowercase(),
        _ => match (part_of(tab), state, tab.bells()) {
            (Some(n), ..) => format!("working, {n} percent"),
            (_, ProgramState::Working, _) => "working".into(),
            (_, _, 1..) => "bell rang".into(),
            _ => "idle".into(),
        },
    };
    format!("{}, {signal}", tab.title())
}

/// The tab's icon of `px`: a server for a tab with a remote space, a terminal else; in the
/// accent while it shows, the third text colour else.
pub(crate) fn tab_icon(tab: Tab, px: f32, on: impl Fn() -> bool + 'static) -> impl View {
    dynamic(move || {
        let remote = tab.spaces.with(|spaces| spaces.iter().any(|it| it.remote.is_some()));
        let body = if remote { icons::SERVER } else { icons::SQUARE_TERMINAL };
        AnyView::new(icon(body).size(px).color(if on() { token::ACCENT } else { token::TEXT_3 }))
    })
}

/// What a tab's second line says, while it shows: the kinds of its spaces and their count.
pub(crate) fn tab_line(tab: Tab) -> String {
    let (n, ssh) =
        tab.spaces.with(|spaces| (spaces.len(), spaces.iter().filter(|it| it.remote.is_some()).count()));
    let local = n - ssh;
    let kinds = match (ssh, local) {
        (0, _) => format!("{local} local"),
        (_, 0) => format!("{ssh} ssh"),
        _ => format!("{ssh} ssh \u{B7} {local} local"),
    };
    format!("{kinds} \u{B7} {n} {}", if n == 1 { "space" } else { "spaces" })
}

/// A bar of 2 px along a tab's lower seam, `bottom` px from its bottom edge (-1: over the
/// line under it), its fill as long as its progress.
pub(crate) fn progress_bar(tab: Tab, bottom: f32) -> impl View {
    let fill =
        div().height(2.0).width(move || Length::Pct(f32::from(part_of(tab).unwrap_or(0)))).bg(move || {
            if matches!(tab.progress(), Some(Progress::Error(_))) { token::ERROR } else { token::ACCENT }
        });
    div()
        .absolute()
        .left(0.0)
        .right(0.0)
        .bottom(bottom)
        .height(2.0)
        .bg(token::LINE_STRONG)
        .display(move || if part_of(tab).is_some() { Display::Flex } else { Display::None })
        .child(fill)
}

/// What a space's header says at its right: its progress in the accent, its state's icon and
/// label in the state's colour (`deploy blocked`), then the link's note (a remote one's
/// latency); with no state and no note, the program's title.
pub(crate) fn header_signal(
    s: &quadra_term::Session,
    prefs: crate::prefs::Prefs,
    title: quadra::Signal<String>,
) -> impl View + use<> {
    let (status, progress, note, name) = (s.status(), s.progress(), s.link_note(), s.status_name());
    // The settings say whether the header shows states.
    let status =
        quadra::memo(move || if prefs.status_shown.get() == 0 { status.get() } else { ProgramState::Idle });
    let mark = dynamic(move || {
        let (Some(face), Some(word)) = (state_icon(status.get(), 12.0), state_word(status.get())) else {
            return AnyView::new(div());
        };
        let color = state_look(status.get()).map_or(token::TEXT_2, |it| it.1);
        let label = move || match name.get() {
            name if name.is_empty() => word.to_string(),
            name => format!("{name} {word}"),
        };
        AnyView::new(
            row().align_items(Align::Center).gap(4.0).text_color(color).child(face).child(text(label)),
        )
    });
    let named = div().child(text(move || {
        let quiet = state_word(status.get()).is_none() && note.with(String::is_empty);
        if quiet { title.get() } else { String::new() }
    }));
    let note = div().font_family(fonts::MONO).child(text(move || note.get()));
    let part = div()
        .font_family(fonts::MONO)
        .text_color(token::ACCENT)
        .child(text(move || progress.get().and_then(percent).map(|n| format!("{n}%")).unwrap_or_default()));
    row().align_items(Align::Center).gap(8.0).shrink(0.0).child(part).child(mark).child(note).child(named)
}
