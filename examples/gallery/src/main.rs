//! The gallery in a real window:
//!
//!     tools/cargo.sh run -p quadra-gallery

use quadra::WindowOptions;
use quadra_gallery::Gallery;
use quadra_kit::dark;

fn main() -> quadra::Result<()> {
    let app = quadra_kit::install(quadra::app()?, &dark())?;
    let options = WindowOptions::new("Quadra Gallery").size(1200.0, 800.0).custom_frame();
    app.keymap(quadra_gallery::keymap()).window(options, || Gallery::new("start", true).view())?.run()
}
