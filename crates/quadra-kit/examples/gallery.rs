//! The kit's components in a real window, a section a component, in either theme:
//!
//!     tools/cargo.sh run -p quadra-kit --example gallery

use std::rc::Rc;

use quadra::{Align, Div, Signal, Theme, WindowOptions, column, row, signal, text, use_context};
use quadra_kit::{Size, Variant, button, dark, light, size, token};

/// A section: its title, then its rows.
fn section(title: &'static str) -> Div {
    let title = text(title).size(size::TEXT_SM.size).color(token::TEXT_2);
    column().gap(8.0).child(title)
}

fn line() -> Div {
    row().gap(8.0).align_items(Align::Center)
}

fn buttons() -> Div {
    let wrap = signal(true);
    let variants = [
        ("Primary", Variant::Primary),
        ("Secondary", Variant::Secondary),
        ("Outline", Variant::Outline),
        ("Ghost", Variant::Ghost),
        ("Danger", Variant::Danger),
        ("Link", Variant::Link),
    ];
    let sizes = [("Extra small", Size::Xs), ("Small", Size::Sm), ("Medium", Size::Md), ("Large", Size::Lg)];
    let by_variant = line().children(variants.map(|(name, variant)| button(name).variant(variant)));
    let by_size = line().children(sizes.map(|(name, size)| button(name).size(size)));
    let off = line().children(variants.map(|(name, variant)| button(name).variant(variant).disabled(true)));
    let toggle = button("Word wrap").variant(Variant::Ghost).toggled(wrap);
    let toggle = toggle.on_click(move || wrap.update(|on| *on = !*on));
    let states = line().child(toggle).child(text(move || if wrap.get() { "on" } else { "off" }));
    section("Buttons").child(by_variant).child(by_size).child(off).child(states)
}

fn main() -> quadra::Result<()> {
    let view = || {
        let theme = use_context::<Signal<Rc<Theme>>>().expect("an app gives its views the theme");
        let is_dark = signal(true);
        let swap = move || {
            is_dark.update(|dark| *dark = !*dark);
            theme.set(Rc::new(if is_dark.get_untracked() { dark() } else { light() }.theme()));
        };
        let name = text(move || if is_dark.get() { "One Dark Pro" } else { "One Light" });
        let name = name.size(size::TEXT_LG.size).weight(size::WEIGHT_STRONG);
        let head = line().child(name).child(button("Other theme").size(Size::Sm).on_click(swap));
        quadra_kit::root().padding(24.0).gap(20.0).child(head).child(buttons())
    };
    let app = quadra_kit::install(quadra::app()?, &dark())?;
    app.window(WindowOptions::new("Quadra gallery").size(640.0, 360.0), view)?.run()
}
