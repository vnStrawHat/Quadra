//! How signals look (specs/19, "Signals"): a tab's mark at the end of its row, the dot of
//! its bell on its icon, the bar of its progress, the second line of its row, the chip of a
//! space's header.

use quadra::{Align, AnyView, Display, Length, View, div, dynamic, row, text};
use quadra_design::{fonts, icon, icons, spinner, token};
use quadra_term::ProgramState;
use quadra_term::vt::Progress;

use crate::model::Tab;
use crate::signals::{percent, state_word};

/// The state of a tab's programs: working spins, blocked asks, done is checked, error
/// crossed; else a bell when one rang unseen.
pub(crate) fn tab_mark(tab: Tab) -> impl View {
    dynamic(move || match tab.status() {
        ProgramState::Working => AnyView::new(spinner().size(12.0)),
        ProgramState::Blocked => {
            AnyView::new(div().font_weight(700).text_color(token::WARN).child(text("?")))
        }
        ProgramState::Done => AnyView::new(icon(icons::CHECK).size(12.0).color(token::OK)),
        ProgramState::Error => AnyView::new(icon(icons::X).size(12.0).color(token::ERROR)),
        ProgramState::Idle if tab.bells() > 0 => {
            AnyView::new(icon(icons::BELL).size(12.0).color(token::TEXT_2))
        }
        ProgramState::Idle => AnyView::new(div()),
    })
}

/// The tab's icon, with a red dot while a bell rang unseen.
pub(crate) fn tab_icon(tab: Tab) -> impl View {
    let dot = div()
        .absolute()
        .top(-2.0)
        .right(-3.0)
        .size(6.0, 6.0)
        .radius(3.0)
        .bg(token::ERROR)
        .display(move || if tab.bells() > 0 { Display::Flex } else { Display::None });
    div().child(icon(icons::TERMINAL).size(14.0).color(token::TEXT_2)).child(dot)
}

/// What a tab's second line says: its state, its progress, else what it holds.
pub(crate) fn tab_line(tab: Tab) -> String {
    if let Some(word) = state_word(tab.status()) {
        return word.to_string();
    }
    if let Some(n) = tab.progress().filter(|_| tab.prefs.progress_tab.get()).and_then(percent) {
        return format!("{n}%");
    }
    let n = tab.spaces.with(Vec::len);
    format!("{n} local \u{B7} {n} {}", if n == 1 { "space" } else { "spaces" })
}

/// A line of 2 px along the bottom of a tab's row, as long as its progress.
pub(crate) fn progress_bar(tab: Tab) -> impl View {
    let part = move || tab.progress().filter(|_| tab.prefs.progress_tab.get()).and_then(percent);
    div()
        .absolute()
        .left(0.0)
        .bottom(0.0)
        .height(2.0)
        .width(move || Length::Pct(f32::from(part().unwrap_or(0))))
        .bg(
            move || {
                if matches!(tab.progress(), Some(Progress::Error(_))) { token::ERROR } else { token::ACCENT }
            },
        )
        .display(move || if part().is_some() { Display::Flex } else { Display::None })
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
