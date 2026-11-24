//! Icons (specs/design, "Space, size, shape"): Lucide outlines on a grid of 24, shown
//! at 10, 12, 14 or 16 px in the colour of the text around them.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use quadra_vector::Svg;
use quadra_view::{DrawingView, drawing};

thread_local! {
    /// The drawings of the bodies shown so far: a body is read once.
    static READ: RefCell<HashMap<&'static str, Rc<Svg>>> = RefCell::default();
}

/// An icon, 14 px square: one of [`crate::icons`], or the body of any drawing made the same
/// way (stroked outlines on a grid of 24).
pub fn icon(body: &'static str) -> DrawingView {
    let read = || {
        let outline = "fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\"";
        Rc::new(Svg::parse(&format!("<svg viewBox=\"0 0 24 24\" {outline}>{body}</svg>")))
    };
    let svg = READ.with(|known| known.borrow_mut().entry(body).or_insert_with(read).clone());
    drawing(svg).size(14.0)
}
