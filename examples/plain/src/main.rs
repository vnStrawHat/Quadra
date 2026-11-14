//! The second kit in a real window:
//!
//!     tools/cargo.sh run -p quadra-plain

use std::rc::Rc;

use quadra::{Color, WindowOptions, row, signal, text};
use quadra_plain::{
    button, checkbox, dialog, labeled, list, radio_group, root, select, slider, switch, tabs,
};

/// A row of things on one middle line.
fn line() -> quadra::Div {
    row().gap(8.0).align_items(quadra::Align::Center)
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
            .child(dialog(
                open,
                "Save the changes?",
                "They are lost otherwise.",
                vec![("No", no), ("Yes", yes)],
            ))
    };
    let options = WindowOptions::new("Quadra Plain").size(520.0, 560.0);
    quadra::app()?.clear_color(Color::rgb(0xFFFFFF)).window(options, view)?.run()
}
