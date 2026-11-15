//! The rows of the expanded SFTP view's panes, and its transfers.

use std::rc::Rc;

use oneterm_ssh::Entry;
use quadra::{Align, AnyView, Length, Role, State, View, by_text, column, div, dynamic, row, text};
use quadra_design::{Tone, badge, fonts, icon, icon_button, icons, progress, token};

use super::{MODIFIED, SIZE};
use crate::panes::{size, when};
use crate::sftp::{Dock, Transfer};

/// A row: its icon, its name (and how it compares), its size and when it changed.
pub(super) fn line(entry: &Entry, mark: Option<&'static str>) -> quadra::Div {
    let shape = if entry.dir { icons::FOLDER } else { icons::FILE };
    let size = if entry.dir { "\u{2014}".to_string() } else { size(entry.size) };
    let mono = |words: String, width: f32| {
        div()
            .width(width)
            .font_family(fonts::MONO)
            .text_size(11.0)
            .text_color(token::TEXT_3)
            .child(text(words))
    };
    let mut name = row()
        .grow(1.0)
        .min_width(Length::Px(0.0))
        .align_items(Align::Center)
        .gap(8.0)
        .child(icon(shape).size(14.0).color(if entry.dir { token::ACCENT } else { token::TEXT_2 }))
        .child(div().min_width(Length::Px(0.0)).child(text(entry.name.clone()).ellipsis()));
    if let Some(mark) = mark {
        name = name.child(badge(mark, if mark == "older" { Tone::Warning } else { Tone::Info }));
    }
    row()
        .align_items(Align::Center)
        .gap(12.0)
        .padding_x(10.0)
        .height(by_text(28.0))
        .radius(4.0)
        .hover(|s| s.bg(token::PANEL_HOVER))
        .when(State::SELECTED, |s| s.bg(token::ACCENT.alpha(0.12)))
        .role(Role::Button)
        .label(entry.name.clone())
        .child(name)
        .child(mono(size, SIZE))
        .child(mono(when(entry.modified), MODIFIED))
}

/// `Transfers`, how many run and ended, then a row each.
pub(super) fn transfers(d: Dock) -> impl View + use<> {
    let list = dynamic(move || {
        d.tick.get();
        let all = d.transfers.get();
        let running = all.iter().filter(|it| it.end.get().is_none()).count();
        let counts = format!("{running} active \u{B7} {} done", all.len() - running);
        let mut list = column().gap(6.0).padding_x(12.0).padding_y(8.0).child(
            row()
                .gap(8.0)
                .text_size(11.0)
                .text_color(token::TEXT_2)
                .child(div().font_weight(600).child(text("TRANSFERS")))
                .child(text(counts)),
        );
        for transfer in all {
            list = list.child(transfer_row(d, transfer));
        }
        AnyView::new(list)
    });
    list.container(
        div()
            .border_top(1.0)
            .border_color(token::LINE)
            .display(crate::locked::shown(move || d.transfers.with(|it| !it.is_empty()))),
    )
}

fn transfer_row(d: Dock, transfer: Rc<Transfer>) -> impl View {
    let (said, part, gone) = (transfer.clone(), transfer.clone(), transfer.clone());
    let arrow = if transfer.up { "\u{2192}" } else { "\u{2190}" };
    let what = format!("{} {arrow} {}", transfer.name, transfer.to);
    let cross = icon_button(icons::X, "Cancel transfer").size(quadra_design::Size::Xs).on_click(move || {
        gone.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        if gone.end.get_untracked().is_some() {
            d.transfers.update(|all| all.retain(|it| !Rc::ptr_eq(it, &gone)));
        }
    });
    row()
        .align_items(Align::Center)
        .gap(12.0)
        .text_size(12.0)
        .child(div().grow(1.0).min_width(Length::Px(0.0)).child(text(what).ellipsis()))
        .child(div().width(200.0).child(progress(move || {
            d.tick.get();
            part.part()
        })))
        .child(div().width(60.0).font_family(fonts::MONO).text_size(11.0).text_color(token::TEXT_2).child(
            text(move || {
                d.tick.get();
                said.state()
            }),
        ))
        .child(cross)
}
