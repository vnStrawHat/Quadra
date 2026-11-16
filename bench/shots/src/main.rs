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
use quadra_design::{dark, light};
use quadra_gallery::{Gallery, PAGES};
use quadra_native_win::{D3d11Renderer, DirectWrite};
use quadra_platform::{
    HeadlessPlatform, Key, Mods, MouseButton, Platform, PlatformEvent, PointerKind, WindowId, WindowOptions,
};
use quadra_prim::{Color, Point, Size};

type Shots = App<D3d11Renderer>;

/// The size of the window the pages are drawn in, as the reference pictures have it.
const SIZE: (f32, f32) = (1200.0, 800.0);

/// The pages that open something, and the test id (or the name) of what opens it: a click,
/// but a hover for the tooltip and the hover card, a right click on the page for the context
/// menu, for the dock a tab torn off and held over the bottom edge of its pane, and for the
/// editor a click in its text and Ctrl-H.
const OPENERS: [(&str, &str); 16] = [
    ("menu", "mn-file"),
    ("select", "sel-ns"),
    ("date-picker", "dp-date"),
    ("color-picker", "cp-accent"),
    ("dialog", "dl-open"),
    ("confirm", "cf-open"),
    ("toast", "to-info"),
    ("palette", "pal-open"),
    ("tooltip", "tt-btn"),
    ("hover-card", "hc-trigger"),
    ("drawer", "drawer-right"),
    ("dock", "main.rs"),
    ("context-menu", ""),
    ("line-chart", "cpu"),
    ("live-table", "live-update"),
    ("editor", "editor"),
];

/// Lets the window live for `ms`: frames, timers, what moves.
fn pass(platform: &HeadlessPlatform, app: &mut Shots, ms: u64) {
    for _ in 0..ms.div_ceil(75) {
        // What the pool decodes is real work in real time: it is waited for.
        quadra_async::wait_idle(Duration::from_secs(10));
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

/// A point 10 px inside the control with this test id, or this name.
fn inside(app: &Shots, window: WindowId, id: &str) -> Option<Point> {
    let tree = app.tree(window)?;
    let nodes = tree.semantic_tree().nodes;
    let found =
        nodes.iter().find(|n| n.semantics.test_id.as_deref() == Some(id) || n.semantics.name == id)?;
    let rect = tree.window_rect(tree.node_of(found.id)?);
    Some(Point::new(rect.x + 10.0, rect.y + 10.0))
}

/// Opens what the page opens. False: the page has nothing with this test id.
fn open(platform: &HeadlessPlatform, app: &mut Shots, window: WindowId, page: &str, opener: &str) -> bool {
    if opener.is_empty() {
        point(platform, window, Point::new(450.0, 260.0), MouseButton::Right);
        return true;
    }
    if page == "editor" {
        point(platform, window, Point::new(520.0, 360.0), MouseButton::Left);
        for down in [true, false] {
            platform.push(
                window,
                PlatformEvent::Key { key: Key::H, code: 0, mods: Mods::CTRL, down, repeat: false },
            );
        }
        return true;
    }
    let Some(at) = inside(app, window, opener) else { return false };
    if page == "dock" {
        let moves = [
            (PointerKind::Down, 0.0, 0.0),
            (PointerKind::Move, 0.0, 20.0),
            (PointerKind::Move, 150.0, 290.0),
        ];
        for (kind, dx, dy) in moves {
            let (button, at) = (MouseButton::Left, Point::new(at.x + dx, at.y + dy));
            platform.push(window, PlatformEvent::Pointer { kind, button, at, mods: Mods::NONE });
        }
        return true;
    }
    let rests = ["tooltip", "hover-card", "line-chart"].contains(&page);
    let button = if rests { MouseButton::None } else { MouseButton::Left };
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
    let theme = args.next().unwrap_or_default();
    let is_dark = theme != "light";
    let only: Vec<String> = args.collect();
    std::fs::create_dir_all(&out)?;

    let platform = HeadlessPlatform::new();
    platform.set_image_decoder(std::sync::Arc::new(quadra_native_win::WicImages));
    let text = DirectWrite::new().map_err(AppError::Text)?;
    let app = App::with(Rc::new(platform.clone()), D3d11Renderer::new(true)?, Box::new(text));
    // `contrast`: a high contrast of black windows and white text (the system's "Night sky").
    let night_sky = quadra_platform::SystemColors {
        window: Color::rgb(0x000000),
        text: Color::rgb(0xFFFFFF),
        highlight: Color::rgb(0xD6B4FD),
        highlight_text: Color::rgb(0x2B2B2B),
        link: Color::rgb(0x8080FF),
        disabled: Color::rgb(0xA6A6A6),
        button: Color::rgb(0x000000),
        button_text: Color::rgb(0xFFFFFF),
    };
    let kit = match theme.as_str() {
        "light" => light(),
        "contrast" => quadra_design::high_contrast(&night_sky),
        _ => dark(),
    };
    let app = quadra_design::install(app, &kit)?;
    let mut app = app.keymap(quadra_gallery::keymap());
    // QUADRA_TEXT draws text that many times larger (specs/12, "Text size"): 1.5 or 2.
    if let Some(text) = std::env::var("QUADRA_TEXT").ok().and_then(|text| text.parse().ok()) {
        let size = quadra_app::TextSize { text, system: quadra_app::SystemText::None, ..Default::default() };
        app = app.text_size(size);
    }
    let shown: Rc<Cell<Option<Gallery>>> = Rc::default();
    let tell = shown.clone();
    let window = app.open_window(WindowOptions::new("shots").size(SIZE.0, SIZE.1), move || {
        let gallery = Gallery::new("start", is_dark);
        tell.set(Some(gallery));
        gallery.view()
    })?;
    let gallery = shown.get().ok_or("the gallery was not built")?;
    // QUADRA_SCALE draws the pages as a display at that scale shows them: 1.5 for 150 %.
    if let Some(scale) = std::env::var("QUADRA_SCALE").ok().and_then(|scale| scale.parse().ok()) {
        platform.resize(window, Size::new(SIZE.0, SIZE.1), scale);
    }
    for page in PAGES.iter().filter(|page| only.is_empty() || only.iter().any(|id| id == page.id)) {
        gallery.page.set(page.id);
        // The page is built and laid out, and what moves has moved for a moment.
        pass(&platform, &mut app, 300);
        save(&mut app, window, out.join(format!("{}.png", page.id)))?;
        let Some((_, opener)) = OPENERS.iter().find(|(id, _)| *id == page.id) else { continue };
        if !open(&platform, &mut app, window, page.id, opener) {
            continue;
        }
        // What opens has opened; the tints of a table that changed are still to be seen.
        pass(&platform, &mut app, if page.id == "live-table" { 300 } else { 750 });
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
