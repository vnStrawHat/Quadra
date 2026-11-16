//! The rows of the expanded SFTP view's panes, and its transfers.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use oneterm_ssh::Entry;
use quadra::{
    Align, AnyView, Justify, Length, Paint, Position, Role, State, View, by_text, column, div, dynamic, row,
    text,
};
use quadra_design::{fonts, icon, icon_button, icons, token};

use super::{MODIFIED, SIZE};
use crate::panes::{day, size};
use crate::sftp::{Dock, Transfer};

/// A value at the right of its column: mono, 12 px, the second colour.
pub(super) fn cell(words: impl Into<String>, width: f32) -> quadra::Div {
    row()
        .width(width)
        .shrink(0.0)
        .justify(Justify::End)
        .font_family(fonts::MONO)
        .text_size(12.0)
        .text_color(token::TEXT_2)
        .child(text(words.into()))
}

/// A row of 26 px with a line under it: its icon, its name (and how it compares), its size and
/// when it changed. Chosen: the accent at 10 % and a bar of it at its left.
pub(super) fn line(
    entry: &Entry,
    mark: Option<&'static str>,
    chosen: impl Fn() -> bool + Clone + 'static,
) -> quadra::Div {
    let (file, picked) = (!entry.dir, chosen.clone());
    let tint = move || -> Paint {
        match (file, picked()) {
            (false, _) => token::HUE_BLUE.into(),
            (true, true) => token::ACCENT.into(),
            (true, false) => token::TEXT_2.into(),
        }
    };
    let shape = if entry.dir { icons::FOLDER } else { icons::FILE };
    let parent = entry.name == "..";
    let size = match (entry.dir, parent) {
        (_, true) => String::new(),
        (true, _) => "\u{2014}".to_string(),
        _ => size(entry.size),
    };
    let mut name = row()
        .grow(1.0)
        .min_width(Length::Px(0.0))
        .align_items(Align::Center)
        .gap(8.0)
        .child(div().text_color(tint).child(icon(shape).size(14.0)))
        .child(div().min_width(Length::Px(0.0)).child(text(entry.name.clone()).ellipsis()));
    if let Some(mark) = mark {
        name = name.child(div().text_size(12.0).text_color(token::HUE_YELLOW).child(text(mark)));
    }
    let picked_too = chosen.clone();
    let bar_paint =
        move || -> Paint { if chosen() { token::ACCENT.into() } else { token::ACCENT.alpha(0.0) } };
    let bar = div().position(Position::Absolute).left(Length::Px(0.0)).top(Length::Px(0.0));
    let bar = bar.bottom(Length::Px(0.0)).width(2.0).bg(bar_paint);
    row()
        .align_items(Align::Center)
        .gap(12.0)
        .padding_x(12.0)
        .height(by_text(26.0))
        .shrink(0.0)
        .border_bottom(1.0)
        .border_color(token::LINE)
        .hover(|s| s.bg(token::PANEL_HOVER))
        .selected(picked_too)
        .when(State::SELECTED, |s| s.bg(token::ACCENT.alpha(0.1)))
        .role(Role::Button)
        .child(bar)
        .label(if parent { "Parent folder".to_string() } else { entry.name.clone() })
        .child(name)
        .child(cell(size, SIZE))
        .child(cell(day(entry.modified), MODIFIED))
}

/// `Transfers`, how many run and ended, the rate of those that run, then a row each.
pub(super) fn transfers(d: Dock) -> impl View + use<> {
    let counts = move || {
        d.tick.get();
        d.transfers.with(|all| {
            let running = all.iter().filter(|it| it.end.get().is_none()).count();
            format!("{running} active \u{B7} {} done", all.len() - running)
        })
    };
    let head = row()
        .align_items(Align::Center)
        .gap(8.0)
        .text_size(12.0)
        .text_color(token::TEXT_2)
        .child(div().font_weight(600).child(text("Transfers")))
        .child(div().grow(1.0).child(text(counts)))
        .child(div().font_family(fonts::MONO).child(text(rate(d))));
    let list = dynamic(move || {
        let mut list = column().gap(6.0);
        for transfer in d.transfers.get() {
            list = list.child(transfer_row(d, transfer));
        }
        AnyView::new(list)
    });
    column()
        .gap(6.0)
        .padding_x(12.0)
        .padding_top(6.0)
        .padding_bottom(10.0)
        .shrink(0.0)
        .bg(token::PANEL)
        .border_top(1.0)
        .border_color(token::LINE)
        .display(crate::locked::shown(move || d.transfers.with(|it| !it.is_empty())))
        .child(head)
        .child(list.container(column()))
}

/// The bytes a second of what runs, `1.2 MB/s`, measured over a second at least.
fn rate(d: Dock) -> impl Fn() -> String + 'static {
    let last = Cell::new((Instant::now(), 0u64));
    let said = Rc::new(std::cell::RefCell::new(String::new()));
    move || {
        d.tick.get();
        let (moved, running) = d.transfers.with(|all| {
            let moved = all.iter().map(|it| it.done.load(Ordering::Relaxed)).sum::<u64>();
            (moved, all.iter().any(|it| it.end.get().is_none()))
        });
        let (then, was) = last.get();
        let secs = then.elapsed().as_secs_f64();
        if !running {
            said.borrow_mut().clear();
        } else if secs >= 1.0 {
            let speed = (moved.saturating_sub(was) as f64 / secs) as u64;
            *said.borrow_mut() = format!("{}/s", size(speed));
        }
        if secs >= 1.0 || !running {
            last.set((Instant::now(), moved));
        }
        said.borrow().clone()
    }
}

fn transfer_row(d: Dock, transfer: Rc<Transfer>) -> impl View {
    let (said, part, gone) = (transfer.clone(), transfer.clone(), transfer.clone());
    let arrow = if transfer.up { icons::ARROW_RIGHT } else { icons::ARROW_LEFT };
    let cross = icon_button(icons::X, "Cancel transfer").size(quadra_design::Size::Xs).on_click(move || {
        gone.cancel.store(true, Ordering::Relaxed);
        if gone.end.get_untracked().is_some() {
            d.transfers.update(|all| all.retain(|it| !Rc::ptr_eq(it, &gone)));
        }
    });
    let what = row()
        .grow(1.0)
        .min_width(Length::Px(0.0))
        .align_items(Align::Center)
        .gap(8.0)
        .child(div().text_color(token::TEXT_2).child(icon(arrow).size(14.0)))
        .child(text(format!("{} to ", transfer.name)))
        .child(
            div()
                .min_width(Length::Px(0.0))
                .font_family(fonts::MONO)
                .text_size(12.0)
                .text_color(token::TEXT_2)
                .child(text(transfer.to.clone()).ellipsis()),
        );
    let fill = div().height(2.0).bg(token::ACCENT).width(move || {
        d.tick.get();
        Length::Pct(part.part() * 100.0)
    });
    row()
        .align_items(Align::Center)
        .gap(12.0)
        .child(what)
        .child(div().width(200.0).height(2.0).shrink(0.0).bg(token::LINE_STRONG).child(fill))
        .child(cell(String::new(), 44.0).child(text(move || {
            d.tick.get();
            said.state()
        })))
        .child(cross)
}
