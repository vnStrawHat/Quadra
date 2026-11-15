//! 1Term in a real window:
//!
//!     tools/cargo.sh run -p oneterm

#![windows_subsystem = "windows"]

use std::rc::Rc;

use oneterm::{NewSession, OneTerm, keymap, local_shell};
use quadra::WindowOptions;

fn main() -> quadra::Result<()> {
    let app = quadra_design::install_system(quadra::app()?)?;
    let options = WindowOptions::new("1Term").size(1440.0, 900.0).fit(0.85).centered().custom_frame();
    let shells: NewSession = Rc::new(|| local_shell(None));
    app.keymap(keymap()).window(options, move || OneTerm::new(shells).view())?.run()
}
