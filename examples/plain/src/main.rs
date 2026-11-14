//! The second kit in a real window:
//!
//!     tools/cargo.sh run -p quadra-plain

use std::rc::Rc;

use quadra::{BorderStyle, Color, Gradient, WindowOptions, row, signal, text};
use quadra_plain::{
    button, checkbox, dialog, labeled, list, radio_group, root, select, slider, switch, tabs,
};

/// A row of things on one middle line.
fn line() -> quadra::Div {
    row().gap(8.0).align_items(quadra::Align::Center)
}

/// What the renderer draws besides flat boxes: gradients, and borders that are not solid.
fn paints() -> quadra::Div {
    let (ink, sky, rose) = (Color::rgb(0x1A1A1A), Color::rgb(0x3B82F6), Color::rgb(0xF43F5E));
    let boxed = || quadra::div().size(56.0, 40.0);
    line()
        .child(boxed().gradient(Gradient::linear(0.0, [sky, rose])))
        .child(boxed().gradient(Gradient::conic(0.0, [sky, rose, sky])).radius(20.0))
        .child(boxed().border(1.0).border_color(ink).border_style(BorderStyle::Dashed))
        .child(boxed().border(2.0).border_color(ink).border_style(BorderStyle::Dotted).radius(8.0))
        // Frosted glass over the two gradients.
        .child(
            quadra::div()
                .absolute()
                .left(28.0)
                .top(8.0)
                .size(64.0, 24.0)
                .bg(Color::WHITE.alpha(0.3))
                .backdrop_blur(6.0),
        )
}

fn main() -> quadra::Result<()> {
    let view = || {
        let (on, lit, size, page) = (signal(true), signal(false), signal(1usize), signal(0usize));
        let (volume, fruit, picked, open) = (signal(40.0f32), signal(0usize), signal(None), signal(false));
        let said = signal(String::from("Nothing asked yet"));
        let yes: Rc<dyn Fn()> = Rc::new(move || said.set("Saved".into()));
        let no: Rc<dyn Fn()> = Rc::new(move || said.set("Not saved".into()));
        let things = ["Paper", "Ink", "Pen", "Ruler", "Eraser"].map(String::from).to_vec();
        root()
            .child(tabs(&["Controls", "About"], page))
            .child(labeled(
                "Button",
                line().child(button("Ask", move || open.set(true))).child(text(move || said.get())),
            ))
            .child(labeled(
                "Toggles",
                row().gap(16.0).child(checkbox(on, "Lines")).child(switch(lit, "Light")),
            ))
            .child(labeled("Size", radio_group(&["Small", "Medium", "Large"], size)))
            .child(labeled(
                "Volume",
                line()
                    .child(slider(volume, (0.0, 100.0), 1.0))
                    .child(text(move || format!("{:.0}", volume.get()))),
            ))
            .child(labeled("Fruit", select(&["Apple", "Pear", "Fig"], fruit)))
            .child(labeled("Things", list(things, picked, 120.0)))
            .child(labeled("Paints", paints()))
            .child(dialog(
                open,
                "Save the changes?",
                "They are lost otherwise.",
                vec![("No", no), ("Yes", yes)],
            ))
    };
    let options = WindowOptions::new("Quadra Plain").size(520.0, 620.0);
    quadra::app()?.clear_color(Color::rgb(0xFFFFFF)).window(options, view)?.run()
}
