//! Draws the pages of the gallery without a window, to pictures, for design review and for
//! comparison with the reference pictures (specs/12, "Snapshots"): the software device, the
//! real text, the headless platform. Nothing shows on the desktop.
//!
//!     tools/cargo.sh run -p quadra-bench-shots -- [out dir] [dark|light] [page ..]

mod png;

use std::cell::Cell;
use std::error::Error;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use quadra_app::{App, AppError};
use quadra_gallery::{Gallery, PAGES};
use quadra_kit::{dark, light};
use quadra_native_win::{D3d11Renderer, DirectWrite};
use quadra_platform::{HeadlessPlatform, Platform, WindowOptions};

/// The size of the window the pages are drawn in, as the reference pictures have it.
const SIZE: (f32, f32) = (1200.0, 800.0);

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().unwrap_or_else(|| ".tmp/shots".into()));
    let is_dark = args.next().is_none_or(|theme| theme != "light");
    let only: Vec<String> = args.collect();
    std::fs::create_dir_all(&out)?;

    let platform = HeadlessPlatform::new();
    let text = DirectWrite::new().map_err(AppError::Text)?;
    let app = App::with(Rc::new(platform.clone()), D3d11Renderer::new(true)?, Box::new(text));
    let mut app = quadra_kit::install(app, &if is_dark { dark() } else { light() })?;
    let shown: Rc<Cell<Option<Gallery>>> = Rc::default();
    let tell = shown.clone();
    let window = app.open_window(WindowOptions::new("shots").size(SIZE.0, SIZE.1), move || {
        let gallery = Gallery::new("start", is_dark);
        tell.set(Some(gallery));
        gallery.view()
    })?;
    let gallery = shown.get().ok_or("the gallery was not built")?;
    for page in PAGES.iter().filter(|page| only.is_empty() || only.iter().any(|id| id == page.id)) {
        gallery.page.set(page.id);
        // The page is built and laid out, and what moves has moved for a moment.
        for _ in 0..4 {
            platform.waker().wake();
            platform.step(&mut app);
            platform.advance(Duration::from_millis(75), &mut app);
        }
        let pixels = app.read_pixels(window)?;
        let file = out.join(format!("{}.png", page.id));
        std::fs::write(&file, png::png(pixels.size.width, pixels.size.height, &pixels.rgba))?;
        println!("{}", file.display());
    }
    Ok(())
}
