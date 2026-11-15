//! The gallery in a real window:
//!
//!     tools/cargo.sh run -p quadra-gallery

use quadra::WindowOptions;
use quadra_gallery::Gallery;

fn main() -> quadra::Result<()> {
    // The theme the system asks for, and the other when the user changes it.
    let app = quadra_design::install_system(quadra::app()?)?;
    let dark = app.appearance().dark;
    let options = WindowOptions::new("Quadra Gallery").size(1200.0, 800.0).custom_frame();
    app.keymap(quadra_gallery::keymap()).window(options, || Gallery::new("start", dark).view())?.run()
}
