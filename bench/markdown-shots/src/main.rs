//! The Markdown app's screens drawn without a window, to pictures (specs/21): the renderer
//! and the text of the system it runs on (the software device on Windows), the headless
//! platform. Nothing shows on the desktop.
//!
//!     tools/cargo.sh run -p quadra-bench-markdown-shots -- [out dir] [scene ..]
//!
//! Scenes: preview, raw, side (the sample document, `sample.md`, in each layout), chosen (side
//! by side, the heading "Code" pressed in the preview: both panes at it, tinted), empty (an
//! untitled window), tables and tables-side (`tables.md`, tables with long text, in Preview and
//! side by side), tabs (the sample and the tables in two tabs), find (Preview's, "the"), logo (the logo at 256 px on the dark and the light canvas), large (the time
//! of a large document's steps, printed; run it in the dist profile), icon (writes
//! `apps/markdown/assets/logo.ico`, and a picture of its sizes), settings and about (the
//! settings window at its first page and at About).

mod icon;
mod large;
#[path = "../../shots/src/png.rs"]
mod png;

use std::cell::RefCell;
use std::error::Error;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use markdown::{MdApp, Model, PREVIEW, RAW, SIDE_BY_SIDE, keymap};
use quadra_app::{App, AppError};
#[cfg(target_os = "macos")]
use quadra_native_mac::{CoreText as Fonts, MetalRenderer as Gpu};
#[cfg(windows)]
use quadra_native_win::{D3d11Renderer as Gpu, DirectWrite as Fonts};
use quadra_platform::{HeadlessPlatform, Platform, WindowOptions};

const SCENES: [(&str, usize); 10] = [
    ("preview", PREVIEW),
    ("raw", RAW),
    ("side", SIDE_BY_SIDE),
    ("chosen", SIDE_BY_SIDE),
    ("empty", PREVIEW),
    ("tables", PREVIEW),
    ("tables-side", SIDE_BY_SIDE),
    ("tabs", PREVIEW),
    ("find", PREVIEW),
    ("find-raw", RAW),
];

/// An app on no display, drawn by the renderer and the text of the system it runs on, as an
/// app of that system (its window buttons, its names of keys).
fn app() -> Result<(HeadlessPlatform, App<Gpu>), Box<dyn Error>> {
    let platform = HeadlessPlatform::new();
    platform.as_the_system();
    let text = Fonts::new().map_err(AppError::Text)?;
    #[cfg(windows)]
    let gpu = Gpu::new(true)?;
    #[cfg(target_os = "macos")]
    let gpu = Gpu::new()?;
    Ok((platform.clone(), App::with(Rc::new(platform), gpu, Box::new(text))))
}

fn pass(platform: &HeadlessPlatform, app: &mut App<Gpu>, ms: u64) {
    for _ in 0..ms.div_ceil(75) {
        quadra_async::wait_idle(Duration::from_secs(10));
        platform.waker().wake();
        platform.step(app);
        platform.advance(Duration::from_millis(75), app);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().unwrap_or_else(|| ".tmp/shots/markdown".into()));
    let only: Vec<String> = args.collect();
    std::fs::create_dir_all(&out)?;
    let sample = std::path::absolute("bench/markdown-shots/src/sample.md")?;
    let tables = std::path::absolute("bench/markdown-shots/src/tables.md")?;
    let wanted = |it: &str| only.is_empty() || only.iter().any(|o| o == it);
    if wanted("logo") {
        logo(&out)?;
    }
    if only.iter().any(|o| o == "icon") {
        icon::icon(&out)?;
    }
    for (scene, first) in [("settings", 0), ("about", 4)] {
        if wanted(scene) {
            settings(&out, scene, first)?;
        }
    }
    if only.iter().any(|o| o == "large") {
        large::large(&out)?;
    }
    for (scene, layout) in SCENES.iter().filter(|(it, _)| wanted(it)) {
        let (platform, app) = app()?;
        let mut app = quadra_design::install(app, &quadra_design::dark())?.keymap(keymap());
        markdown::plain_code(&mut app);
        let built: Rc<RefCell<Option<Model>>> = Rc::default();
        let document = if scene.starts_with("tables") { tables.clone() } else { sample.clone() };
        let mut files: Vec<PathBuf> = (*scene != "empty").then_some(document).into_iter().collect();
        if *scene == "tabs" {
            files.push(tables.clone());
        }
        let tell = built.clone();
        let window = app.open_window(WindowOptions::new("Markdown").size(1280.0, 860.0), move || {
            let md = MdApp::with_files(files.clone());
            *tell.borrow_mut() = Some(md.model.clone());
            md.view()
        })?;
        let model = built.borrow().clone().ok_or("no model")?;
        pass(&platform, &mut app, 150);
        model.layout.set(*layout);
        pass(&platform, &mut app, 300);
        if *scene == "chosen" {
            let line = model.text.get_untracked().lines().position(|it| it == "## Code").unwrap_or(0);
            let top = model.places.with_untracked(|p| p.iter().find(|it| it.line == line).map(|it| it.top));
            model.driver.set(markdown::Driver::Preview);
            model.preview_y.set(top.unwrap_or(0.0) + 24.0);
            model.flash.set(Some(line));
        }
        if *scene == "find-raw" {
            model.editor.run(quadra_code::EditorCommand::Find);
        }
        if *scene == "find" {
            model.search.text.set("the".into());
            model.search.open.set(true);
            pass(&platform, &mut app, 100);
            model.search.current.set(1);
        }
        pass(&platform, &mut app, 300);
        let pixels = app.read_pixels(window)?;
        let file = out.join(format!("{scene}.png"));
        std::fs::write(&file, png::png(pixels.size.width, pixels.size.height, &pixels.rgba))?;
        println!("{}", file.display());
    }
    Ok(())
}

/// The logo at 256 px on the dark canvas and on the light, side by side.
fn logo(out: &std::path::Path) -> Result<(), Box<dyn Error>> {
    let (platform, app) = app()?;
    let mut app = quadra_design::install(app, &quadra_design::dark())?;
    let window = app.open_window(WindowOptions::new("Logo").size(640.0, 320.0), || {
        let tile = |bg: quadra_prim::Color| {
            quadra_view::column()
                .grow(1.0)
                .align_items(quadra_layout::Align::Center)
                .justify(quadra_layout::Justify::Center)
                .bg(bg)
                .child(markdown::logo(256.0))
        };
        quadra_view::row()
            .grow(1.0)
            .child(tile(quadra_prim::Color::rgb(0x1e2227)))
            .child(tile(quadra_prim::Color::WHITE))
    })?;
    pass(&platform, &mut app, 300);
    let pixels = app.read_pixels(window)?;
    let file = out.join("logo.png");
    std::fs::write(&file, png::png(pixels.size.width, pixels.size.height, &pixels.rgba))?;
    println!("{}", file.display());
    Ok(())
}

/// The settings window, at its page `first`.
fn settings(out: &std::path::Path, scene: &str, first: usize) -> Result<(), Box<dyn Error>> {
    let (platform, app) = app()?;
    let mut app = quadra_design::install(app, &quadra_design::dark())?;
    let window = app.open_window(WindowOptions::new("Markdown Settings").size(980.0, 680.0), move || {
        let model = Model::new();
        quadra_design::root().child(markdown::settings_page(&model, first))
    })?;
    pass(&platform, &mut app, 300);
    let pixels = app.read_pixels(window)?;
    let file = out.join(format!("{scene}.png"));
    std::fs::write(&file, png::png(pixels.size.width, pixels.size.height, &pixels.rgba))?;
    println!("{}", file.display());
    Ok(())
}
