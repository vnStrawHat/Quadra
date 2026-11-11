//! A text input, a button and a menu in a real window:
//!
//!     tools/cargo.sh run -p quadra --example form

use quadra::{
    Along, Color, NodeRef, Place, Role, Side, WindowOptions, column, div, overlay, row, show, signal, text,
    text_input,
};

fn main() -> quadra::Result<()> {
    let view = || {
        let (name, open) = (signal("Quadra".to_string()), signal(false));
        let field = div().width(240.0).padding(6.0).radius(4.0).border(1.0);
        let field =
            field.border_color(Color::rgb(0xB0B0B0)).focus_within(|s| s.border_color(Color::rgb(0x3366FF)));
        let hello = text(move || format!("Hello, {}!", name.get())).size(18.0);

        // A button that opens a menu below itself; an item of the menu writes the name.
        let anchor = NodeRef::new();
        let button = div().role(Role::Button).expanded(move || open.get()).focusable();
        let button = button.padding_x(12.0).padding_y(6.0).radius(4.0).bg(Color::rgb(0xE8E8E8));
        let button = button.hover(|s| s.bg(Color::rgb(0xD8D8D8))).active(|s| s.bg(Color::rgb(0xC4C4C4)));
        let button =
            button.node_ref(&anchor).on_click(move |_| open.update(|o| *o = !*o)).child(text("Names"));
        let menu = move || {
            let item = move |label: &'static str| {
                let pick = move |_: &mut quadra::PointerEvent| {
                    name.set(label.to_string());
                    open.set(false);
                };
                let item = div().role(Role::MenuItem).width(140.0).padding_x(10.0).padding_y(5.0).radius(3.0);
                item.hover(|s| s.bg(Color::rgb(0xDCE6FF))).on_click(pick).child(text(label))
            };
            let items = column().role(Role::Menu).padding(4.0).radius(6.0).border(1.0);
            let items = items.border_color(Color::rgb(0xC0C0C0));
            let items = items.bg(Color::WHITE).children(["Quadra", "Việt Nam", "World"].map(item));
            let place = Place::Beside(anchor.clone(), Side::Below, Along::Start, 4.0);
            overlay(place, items).on_outside_press(move || open.set(false))
        };

        let line = row().gap(8.0).align_items(quadra::Align::Center);
        let line = line.child(field.child(text_input(name))).child(button);
        let hint = text("Tab, arrows, Ctrl+Z, Ctrl+C and Ctrl+V work; click to place the caret.").size(12.0);
        let page = column().padding(24.0).gap(12.0).bg(Color::WHITE);
        page.child(hello).child(line).child(hint).child(show(move || open.get(), menu))
    };
    quadra::app()?.window(WindowOptions::new("Quadra form").size(460.0, 240.0), view)?.run()
}
