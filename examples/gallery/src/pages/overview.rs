//! The first pages: what Quadra is, and what its design language is made of.

use quadra::{Div, column, div, row, text};
use quadra_kit::{kbd, size, token};

use crate::common::{demo, hint, line, mono, page};

pub fn start_page() -> Div {
    let principles = [
        ("Quiet chrome, loud content", "Panels and toolbars recede; code, logs and data get full contrast."),
        (
            "Planes, not boxes",
            "Regions differ by one step of tone and a hairline. Only overlays cast shadows.",
        ),
        ("Dense with rhythm", "Compact by default, on a 4 px grid and four control heights: 18, 22, 28, 32."),
        ("Mono is first-class", "Identifiers, numbers, paths and durations use the mono face."),
        ("Status by color and shape", "Every status has a shape too, so it reads in grayscale."),
        (
            "One accent",
            "The accent colors text and icons: toggled, matched, linked. Selection is a neutral step.",
        ),
    ];
    let mut list = column();
    for (i, (title, about)) in principles.into_iter().enumerate() {
        let words = column().gap(2.0).child(text(title).weight(size::WEIGHT_STRONG)).child(hint(about));
        let item = row().gap(16.0).padding_y(10.0).border_bottom(1.0).border_color(token::LINE);
        list = list.child(item.child(div().width(24.0).child(mono(format!("0{}", i + 1)))).child(words));
    }
    let keys = [
        ("ctrl-k", "commands"),
        ("ctrl-shift-p", "profiler"),
        ("ctrl-shift-f", "paint flashing"),
        ("ctrl-shift-i", "inspector"),
    ];
    let mut shortcuts = line();
    for (keys, does) in keys {
        shortcuts = shortcuts.child(kbd(keys)).child(hint(does));
    }
    let about = "A native, GPU-rendered UI framework in Rust. Everything here is drawn by Quadra itself, \
                 in the One design language: built for terminals, editors and cluster tools that people \
                 keep open all day.";
    page(
        "Quadra",
        about,
        [
            demo(
                "Hello, world",
                text("Xin ch\u{E0}o! Pick a component on the left."),
                "div().padding(16.0).child(text(\"Xin ch\u{E0}o\"))",
            ),
            demo("Design principles", list, ""),
            demo("Developer shortcuts", shortcuts, ""),
        ],
    )
}
