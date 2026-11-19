//! The `large` scene: what a large document costs (specs/21, "Its cost"). Documents of about
//! 1 000 and 10 000 lines (headings, prose, lists, tables of long text, code) are opened in
//! the app, and the time of each step is printed, from asking to the last frame it made being
//! presented (the frames then stand still): opening in Preview, a layout changed to side by
//! side, a turn of the wheel there, a letter typed until the preview shows it. Each step is
//! checked to have shown what it did.

use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;
use std::time::{Duration, Instant};

use markdown::{Driver, MdApp, Model, SIDE_BY_SIDE, keymap};
use quadra_app::{App, AppError};
use quadra_native_win::{D3d11Renderer, DirectWrite};
use quadra_platform::{HeadlessPlatform, Platform, WindowOptions};

/// A section of 25 lines.
fn section(k: usize) -> String {
    format!(
        "## Section {k}\n\nThe text of section {k}, a paragraph long enough to wrap on a line or two of the \
         preview, with **strong** words, `code` and a [link](https://example.com/{k}).\n\n- one item\n- \
         another item, longer, so that it wraps in a narrow pane\n- [x] a task done\n\n| Setting | Notes \
         |\n|---|---|\n| retries | How many times a request is sent again before the app gives up and \
         says why |\n| timeout | 30 s |\n\n```rust\nfn section_{k}() -> usize {{\n    {k}\n}}\n```\n\n> A \
         quote under section {k}.\n\nOne more paragraph.\n\n"
    )
}

/// The frames presented, and when the last was.
type Presented = Rc<Cell<(u64, Instant)>>;

/// Passes, the window's clock moved 75 ms between them, for `ms` at least and then until two
/// in a row present nothing; the time from `start` to the last frame presented.
fn settle(platform: &HeadlessPlatform, app: &mut App<D3d11Renderer>, seen: &Presented, ms: u64) -> Duration {
    let start = Instant::now();
    let (mut still, mut passes) = (0, 0u64);
    while passes * 75 < ms || still < 2 {
        let before = seen.get().0;
        quadra_async::wait_idle(Duration::from_secs(30));
        platform.waker().wake();
        platform.step(app);
        platform.advance(Duration::from_millis(75), app);
        still = if seen.get().0 == before { still + 1 } else { 0 };
        passes += 1;
    }
    seen.get().1.saturating_duration_since(start)
}

pub fn large(out: &std::path::Path) -> Result<(), Box<dyn Error>> {
    for sections in [40, 400] {
        let text: String = (0..sections).map(section).collect();
        let lines = text.lines().count();
        let file = out.join(format!("large-{sections}.md"));
        std::fs::write(&file, &text)?;
        let platform = HeadlessPlatform::new();
        let words = DirectWrite::new().map_err(AppError::Text)?;
        let app = App::with(Rc::new(platform.clone()), D3d11Renderer::new(true)?, Box::new(words));
        let seen: Presented = Rc::new(Cell::new((0, Instant::now())));
        let told = seen.clone();
        let app = app.on_presented(move |_| told.set((told.get().0 + 1, Instant::now())));
        let mut app = quadra_design::install(app, &quadra_design::dark())?.keymap(keymap());
        markdown::plain_code(&mut app);
        let built: Rc<RefCell<Option<Model>>> = Rc::default();
        let tell = built.clone();
        let start = Instant::now();
        app.open_window(WindowOptions::new("Markdown").size(1280.0, 860.0), move || {
            let md = MdApp::new(Some(file.clone()));
            *tell.borrow_mut() = Some(md.model.clone());
            md.view()
        })?;
        settle(&platform, &mut app, &seen, 0);
        let opened = seen.get().1 - start;
        let m = built.borrow().clone().ok_or("no model")?;
        let places = m.places.with_untracked(Vec::len);
        m.layout.set(SIDE_BY_SIDE);
        let switched = settle(&platform, &mut app, &seen, 0);
        let (driver, y) = (m.driver, m.preview_y.get_untracked());
        driver.set(Driver::Editor);
        m.pos.update(|at| *at += 30.0);
        let wheel = settle(&platform, &mut app, &seen, 0);
        let followed = m.preview_y.get_untracked() > y;
        let middle = lines / 2;
        m.editor.edit(move |w| {
            let at = w.doc.buffer.line_start(middle);
            w.doc.set_cursor(at, at, false);
            w.doc.insert_text("x", true);
        });
        // The preview takes the text 120 ms after the last edit.
        let typed = settle(&platform, &mut app, &seen, 225);
        let shown = m.text.with_untracked(|it| it.lines().nth(middle).is_some_and(|it| it.starts_with('x')));
        // Back to Preview, and to Side by side again: the panes are kept, only laid out.
        m.layout.set(markdown::PREVIEW);
        let back = settle(&platform, &mut app, &seen, 0);
        m.layout.set(SIDE_BY_SIDE);
        let again = settle(&platform, &mut app, &seen, 0);
        let ms = |d: Duration| d.as_secs_f64() * 1000.0;
        println!(
            "large: {lines} lines ({places} blocks placed): open {:.0} ms, side by side {:.0} ms, the editor moved and the preview following {:.1} ms (followed: {followed}), a letter to the preview {:.0} ms (shown: {shown}), back to Preview {:.1} ms, side by side again {:.1} ms, {} frames",
            ms(opened),
            ms(switched),
            ms(wheel),
            ms(typed),
            ms(back),
            ms(again),
            seen.get().0,
        );
    }
    Ok(())
}
