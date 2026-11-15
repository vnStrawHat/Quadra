//! How signals look (specs/19, "Signals"): a tab's mark at the end of its row, the dot of
//! its bell on its icon, the bar of its progress, the second line of its row, the chip of a
//! space's header.

use quadra::{Align, AnyView, Display, Justify, Length, Role, View, div, dynamic, row, text};
use quadra_design::{fonts, icon, icons, token};
use quadra_term::ProgramState;
use quadra_term::vt::Progress;

use crate::model::Tab;
use crate::signals::{percent, state_word};

/// A filled circle of `size` px holding `inside`, read as `word`.
fn disc(size: f32, fill: quadra::Token, word: &'static str, inside: impl View + 'static) -> quadra::Div {
    row()
        .size(size, size)
        .radius(size / 2.0)
        .shrink(0.0)
        .align_items(Align::Center)
        .justify(Justify::Center)
        .bg(fill)
        .text_color(token::TEXT_ON_STATUS)
        .role(Role::Image)
        .label(word)
        .child(inside)
}

/// The part done of a tab's progress, when the settings show it on tabs.
fn part_of(tab: Tab) -> Option<u8> {
    tab.progress().filter(|_| tab.prefs.progress_tab.get()).and_then(percent)
}

/// The state of a tab's programs: blocked asks, done is checked, error crossed, working says
/// so with its progress; else a bell when one rang unseen, else `idle`.
pub(crate) fn tab_mark(tab: Tab) -> impl View {
    dynamic(move || {
        let word = state_word(tab.status()).unwrap_or("idle");
        let small = || div().text_size(11.0).text_color(token::TEXT_2);
        match tab.status() {
            ProgramState::Blocked => AnyView::new(disc(
                18.0,
                token::WARN,
                word,
                div().text_size(11.0).font_weight(700).child(text("?")),
            )),
            ProgramState::Done => AnyView::new(disc(
                16.0,
                token::OK,
                word,
                icon(icons::CHECK).size(10.0).color(token::TEXT_ON_STATUS),
            )),
            ProgramState::Error => AnyView::new(disc(
                16.0,
                token::ERROR,
                word,
                icon(icons::X).size(10.0).color(token::TEXT_ON_STATUS),
            )),
            state if state == ProgramState::Working || part_of(tab).is_some() => {
                let words = small().text_size(10.0).display(move || match tab.status() {
                    ProgramState::Working => Display::Flex,
                    _ => Display::None,
                });
                let part = div().font_family(fonts::MONO).text_size(11.0).text_color(token::ACCENT);
                AnyView::new(
                    row().align_items(Align::Center).gap(6.0).child(words.child(text("working"))).child(
                        part.child(text(move || part_of(tab).map(|n| format!("{n}%")).unwrap_or_default())),
                    ),
                )
            }
            _ if tab.bells() > 0 => AnyView::new(
                div()
                    .role(Role::Image)
                    .label("Bell rang")
                    .child(icon(icons::BELL).size(14.0).color(token::ERROR)),
            ),
            _ => AnyView::new(small().child(text("idle"))),
        }
    })
}

/// The tab's icon: a window for a tab with a remote space, a terminal else; in the accent
/// while it shows, with a red dot while a bell rang unseen.
pub(crate) fn tab_icon(tab: Tab, on: impl Fn() -> bool + 'static) -> impl View {
    let dot = div()
        .absolute()
        .top(-3.0)
        .right(-3.0)
        .size(8.0, 8.0)
        .radius(4.0)
        .border(2.0)
        .border_color(token::PANEL)
        .bg(token::ERROR)
        .display(move || if tab.bells() > 0 { Display::Flex } else { Display::None });
    let face = dynamic(move || {
        let remote = tab.spaces.with(|spaces| spaces.iter().any(|it| it.remote.is_some()));
        let body = if remote { icons::APP_WINDOW } else { icons::SQUARE_TERMINAL };
        AnyView::new(icon(body).size(16.0).color(if on() { token::ACCENT } else { token::TEXT_2 }))
    });
    div().shrink(0.0).child(face).child(dot)
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

/// A bar of 3 px under a tab's row, its fill as long as its progress.
pub(crate) fn progress_bar(tab: Tab) -> impl View {
    let fill =
        div().height(3.0).width(move || Length::Pct(f32::from(part_of(tab).unwrap_or(0)))).bg(move || {
            if matches!(tab.progress(), Some(Progress::Error(_))) { token::ERROR } else { token::ACCENT }
        });
    div()
        .height(3.0)
        .radius(2.0)
        .bg(token::LINE)
        .display(move || if part_of(tab).is_some() { Display::Flex } else { Display::None })
        .child(fill)
}

/// What a space's header says at its right: a chip of its state, its progress.
pub(crate) fn header_signal(
    status: quadra::Signal<ProgramState>,
    progress: quadra::Signal<Option<Progress>>,
    prefs: crate::prefs::Prefs,
) -> impl View {
    // The settings say whether the header shows states.
    let status =
        quadra::memo(move || if prefs.status_shown.get() == 0 { status.get() } else { ProgramState::Idle });
    let chip = div()
        .padding_x(6.0)
        .radius(3.0)
        .bg(move || match status.get() {
            ProgramState::Error => token::ERROR_TINT,
            ProgramState::Blocked => token::WARN_TINT,
            ProgramState::Done => token::OK_TINT,
            _ => token::ACCENT_TINT,
        })
        .display(move || if state_word(status.get()).is_some() { Display::Flex } else { Display::None })
        .child(text(move || state_word(status.get()).unwrap_or("").to_string()));
    let part = text(move || progress.get().and_then(percent).map(|n| format!("{n}%")).unwrap_or_default());
    row().align_items(Align::Center).gap(6.0).font_family(fonts::MONO).child(part).child(chip)
}
