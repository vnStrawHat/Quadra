//! The kit's two themes on its own fonts, in a real window; a click changes the theme:
//!
//!     tools/cargo.sh run -p quadra-kit --example themes

use std::rc::Rc;

use quadra::{Signal, Theme, WindowOptions, column, div, row, signal, text, use_context};
use quadra_kit::{dark, fonts, light, size, token};

fn main() -> quadra::Result<()> {
    let view = || {
        let theme = use_context::<Signal<Rc<Theme>>>().expect("an app gives its views the theme");
        let is_dark = signal(true);
        let swap = move |_: &mut quadra::PointerEvent| {
            is_dark.update(|dark| *dark = !*dark);
            let next = if is_dark.get_untracked() { dark() } else { light() };
            theme.set(Rc::new(next.theme()));
        };
        let swatch = |role, name: &'static str| {
            let chip = div().size(28.0, 28.0).radius(token::RADIUS_MD).border(1.0).border_color(token::LINE);
            row().gap(8.0).align_items(quadra::Align::Center).child(chip.bg(role)).child(text(name))
        };
        let roles = column().gap(6.0);
        let roles = roles.child(swatch(token::PANEL, "panel")).child(swatch(token::RAISED, "raised"));
        let roles = roles.child(swatch(token::ACCENT, "accent")).child(swatch(token::ERROR, "error"));
        let title = text(move || if is_dark.get() { "One Dark Pro" } else { "One Light" });
        let title = title.size(size::TEXT_XL.size).weight(size::WEIGHT_STRONG);
        let muted = text("Click anywhere for the other theme.").color(token::TEXT_2);
        let code = div().font_family(fonts::MONO).child(text("let answer = 42; // Lilex"));
        let page = quadra_kit::root().padding(24.0).gap(12.0).on_click(swap);
        page.child(title).child(muted).child(roles).child(code)
    };
    let app = quadra_kit::install(quadra::app()?, &dark())?;
    app.window(WindowOptions::new("Quadra themes").size(360.0, 300.0), view)?.run()
}
