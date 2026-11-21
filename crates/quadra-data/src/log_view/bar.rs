//! The log view's bar: the filter, its switches, the counts, the Warn and Follow toggles.

use quadra_design::{Size, Variant, button, input, token, tooltip};
use quadra_layout::Align;
use quadra_reactive::Signal;
use quadra_view::{Div, View, by_text, div, row, text};

use crate::log::Log;

/// What the bar sets: the filter's text, whether it is a pattern, whether other lines are
/// dimmed, and whether only warnings and graver are listed.
#[derive(Clone, Copy)]
pub(super) struct Filter {
    pub(super) query: Signal<String>,
    pub(super) pattern: Signal<bool>,
    pub(super) dim: Signal<bool>,
    pub(super) graver: Signal<bool>,
}

/// A small button of the bar that is on or off, with what it does when the pointer rests.
fn toggle(label: &str, tip: &str, on: Signal<bool>) -> impl View {
    let button = button(label.to_string()).variant(Variant::Ghost).size(Size::Sm);
    tooltip(tip, button.toggled(move || on.get()).on_click(move || on.update(|on| *on = !*on)))
}

pub(super) fn bar(log: &Log, filter: Filter, wrong: Signal<bool>, follow: Signal<bool>) -> Div {
    let (all, shown, Filter { query, pattern, dim, graver }) = (log.clone(), log.clone(), filter);
    let counts = text(move || match (wrong.get(), query.with(String::is_empty) || dim.get()) {
        (true, _) => "Bad pattern".to_string(),
        (_, true) => format!("{} lines", all.len()),
        _ => format!("{} of {}", shown.listed(), all.len()),
    });
    let bar = row().align_items(Align::Center).gap(4.0).padding_x(8.0).height(by_text(45.0)).shrink(0.0);
    let bar = bar.border_bottom(1.0).border_color(token::LINE);
    let bar = bar.child(input(query).placeholder("Filter").width(240.0).invalid(wrong));
    let bar = bar.child(toggle(".*", "Regular expression", pattern));
    let bar = bar.child(toggle("Dim", "Dim other lines instead of hiding them", dim));
    let bar = bar.child(div().grow(1.0)).child(div().text_size(12.0).text_color(token::TEXT_3).child(counts));
    let bar = bar.child(toggle("Warn", "Only warnings and errors", graver));
    bar.child(toggle("Follow", "Keep the newest line in view", follow))
}
