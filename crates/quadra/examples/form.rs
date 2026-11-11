//! A text input and what it holds, in a real window:
//!
//!     tools/cargo.sh run -p quadra --example form

use quadra::{Color, WindowOptions, column, div, signal, text, text_input};

fn main() -> quadra::Result<()> {
    let view = || {
        let name = signal("Quadra".to_string());
        let field = div().width(240.0).padding(6.0).radius(4.0).border(1.0);
        let field =
            field.border_color(Color::rgb(0xB0B0B0)).focus_within(|s| s.border_color(Color::rgb(0x3366FF)));
        let hello = text(move || format!("Hello, {}!", name.get())).size(18.0);
        let page = column().padding(24.0).gap(12.0).bg(Color::WHITE);
        page.child(hello)
            .child(field.child(text_input(name)))
            .child(text("Tab, arrows, Ctrl+Z work; click to place the caret.").size(12.0))
    };
    quadra::app()?.window(WindowOptions::new("Quadra form").size(420.0, 180.0), view)?.run()
}
