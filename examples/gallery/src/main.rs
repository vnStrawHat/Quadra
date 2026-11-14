//! The gallery in a real window:
//!
//!     tools/cargo.sh run -p quadra-gallery

use quadra::WindowOptions;
use quadra_gallery::{Gallery, OpenPalette};
use quadra_kit::dark;

fn main() -> quadra::Result<()> {
    let app = quadra_kit::install(quadra::app()?, &dark())?;
    // Ctrl-K opens the command palette, wherever the focus is.
    let mut keys = quadra_kit::keymap();
    keys.bind("ctrl-k", OpenPalette)?;
    quadra_code::bind_keys(&mut keys);
    let options = WindowOptions::new("Quadra Gallery").size(1200.0, 800.0).custom_frame();
    app.keymap(keys).window(options, || Gallery::new("start", true).view())?.run()
}
