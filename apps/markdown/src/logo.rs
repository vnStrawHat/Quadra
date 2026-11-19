//! The logo (specs/21, "Logo"): the mark of Markdown, drawn as if still under construction.

use std::rc::Rc;

use quadra_vector::Svg;
use quadra_view::{DrawingView, drawing};

/// The drawing, a 64-unit square.
pub const LOGO: &str = include_str!("../assets/logo.svg");

thread_local! {
    static READ: Rc<Svg> = Rc::new(Svg::parse(LOGO));
}

/// The logo, `px` square.
pub fn logo(px: f32) -> DrawingView {
    drawing(READ.with(Rc::clone)).size(px)
}
