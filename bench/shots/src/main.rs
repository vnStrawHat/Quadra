//! Draws the pages of the gallery without a window, to pictures, for design review and for
//! comparison with the reference pictures (specs/12, "Snapshots"): the software device, the
//! real text, the headless platform. Nothing shows on the desktop. A page that opens something
//! is drawn a second time with it open (`<page>-open.png`).
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
use quadra_platform::{
    HeadlessPlatform, Key, Mods, MouseButton, Platform, PlatformEvent, PointerKind, WindowId, WindowOptions,
};
use quadra_prim::Point;

type Shots = App<D3d11Renderer>;

/// The size of the window the pages are drawn in, as the reference pictures have it.
const SIZE: (f32, f32) = (1200.0, 800.0);

/// The pages that open something, and the test id of what opens it: a click, but a hover for
/// the tooltip, and a right click on the page for the context menu.
const OPENERS: [(&str, &str); 9] = [
    ("menu", "mn-file"),
    ("select", "sel-ns"),
    ("date-picker", "dp-date"),
    ("dialog", "dl-open"),
    ("confirm", "cf-open"),
    ("toast", "to-info"),
    ("palette", "pal-open"),
    ("tooltip", "tt-btn"),
    ("context-menu", ""),
];

/// Lets the window live for `ms`: frames, timers, what moves.
fn pass(platform: &HeadlessPlatform, app: &mut Shots, ms: u64) {
    for _ in 0..ms.div_ceil(75) {
        platform.waker().wake();
        platform.step(app);
        platform.advance(Duration::from_millis(75), app);
    }
}

fn point(platform: &HeadlessPlatform, window: WindowId, at: Point, button: MouseButton) {
    let kinds: &[PointerKind] = match button {
        MouseButton::None => &[PointerKind::Move],
        _ => &[PointerKind::Move, PointerKind::Down, PointerKind::Up],
    };
    for &kind in kinds {
        let button = if kind == PointerKind::Move { MouseButton::None } else { button };
        platform.push(window, PlatformEvent::Pointer { kind, button, at, mods: Mods::NONE });
    }
}

/// A point 10 px inside the control with this test id.
fn inside(app: &Shots, window: WindowId, id: &str) -> Option<Point> {
    let tree = app.tree(window)?;
    let nodes = tree.semantic_tree().nodes;
    let found = nodes.iter().find(|node| node.semantics.test_id.as_deref() == Some(id))?;
    let rect = tree.window_rect(tree.node_of(found.id)?);
    Some(Point::new(rect.x + 10.0, rect.y + 10.0))
}

/// Opens what the page opens. False: the page has nothing with this test id.
fn open(platform: &HeadlessPlatform, app: &mut Shots, window: WindowId, page: &str, opener: &str) -> bool {
    if opener.is_empty() {
        point(platform, window, Point::new(450.0, 260.0), MouseButton::Right);
        return true;
    }
    let Some(at) = inside(app, window, opener) else { return false };
    let button = if page == "tooltip" { MouseButton::None } else { MouseButton::Left };
    point(platform, window, at, button);
    if let Some(at) = inside(app, window, "to-err").filter(|_| page == "toast") {
        pass(platform, app, 75);
        point(platform, window, at, MouseButton::Left);
    }
    true
}

fn save(app: &mut Shots, window: WindowId, file: PathBuf) -> Result<(), Box<dyn Error>> {
    let pixels = app.read_pixels(window)?;
    std::fs::write(&file, png::png(pixels.size.width, pixels.size.height, &pixels.rgba))?;
    println!("{}", file.display());
    Ok(())
}

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
        pass(&platform, &mut app, 300);
        save(&mut app, window, out.join(format!("{}.png", page.id)))?;
        let Some((_, opener)) = OPENERS.iter().find(|(id, _)| *id == page.id) else { continue };
        if !open(&platform, &mut app, window, page.id, opener) {
            continue;
        }
        pass(&platform, &mut app, 750);
        save(&mut app, window, out.join(format!("{}-open.png", page.id)))?;
        // Closed again, the pointer away, and the notices gone, before the next page.
        let key =
            |down| PlatformEvent::Key { key: Key::Escape, code: 0, mods: Mods::NONE, down, repeat: false };
        platform.push(window, key(true));
        platform.push(window, key(false));
        let (kind, button, at) = (PointerKind::Leave, MouseButton::None, Point::ZERO);
        platform.push(window, PlatformEvent::Pointer { kind, button, at, mods: Mods::NONE });
        pass(&platform, &mut app, 11_000);
    }
    Ok(())
}
