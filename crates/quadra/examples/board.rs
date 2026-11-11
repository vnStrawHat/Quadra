//! Cards dragged between two columns in a real window:
//!
//!     tools/cargo.sh run -p quadra --example board

use quadra::{Color, Cursor, Signal, WindowOptions, column, div, each, row, signal, text};

/// What a drag of a card carries: the card's name.
struct Card(String);

fn main() -> quadra::Result<()> {
    let view = || {
        let todo = signal(vec!["Write the spec".to_string(), "Build it".to_string(), "Ship".to_string()]);
        let done = signal(Vec::<String>::new());
        let card = |name: Signal<String>| {
            let face = || div().width(180.0).padding(8.0).radius(6.0).bg(Color::rgb(0xF1F3F5));
            let preview = move || face().bg(Color::rgb(0xDCE6FF)).child(text(name));
            let card =
                face().cursor(Cursor::Move).draggable_with(move || Card(name.get_untracked()), preview);
            card.hover(|s| s.bg(Color::rgb(0xE3E6EA))).child(text(name))
        };
        // A column takes a card from wherever it is, and puts it at its own end.
        let lane = move |title: &'static str, cards: Signal<Vec<String>>| {
            let take = move |card: &Card, _| {
                for from in [todo, done] {
                    from.update(|names| names.retain(|name| *name != card.0));
                }
                cards.update(|names| names.push(card.0.clone()));
            };
            let lane = column().width(212.0).height(240.0).padding(8.0).gap(6.0).radius(8.0).border(1.0);
            let lane = lane.border_color(Color::rgb(0xD0D4D9)).on_drop(take);
            let lane = lane.drop_target(|s| s.bg(Color::rgb(0xEEF4FF)).border_color(Color::rgb(0x3366FF)));
            lane.child(text(title).size(12.0)).child(each(move || cards.get(), String::clone, card))
        };
        let board = row().padding(24.0).gap(16.0).bg(Color::WHITE);
        board.child(lane("To do", todo)).child(lane("Done", done))
    };
    quadra::app()?.window(WindowOptions::new("Quadra board").size(500.0, 300.0), view)?.run()
}
