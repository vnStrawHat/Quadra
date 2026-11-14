//! Comparison scenes (docs/benchmarks.md): the four small apps other frameworks publish
//! numbers for, with the same text, sizes and frame recording as the V build's. The scene is
//! the first argument; the runner starts the app and measures it from outside.
//!
//!   counter   360x260  a number and three buttons, then idle
//!   list      800x600  a virtual list of 100,000 rows of three texts, scrolling 8 px a frame
//!   grid      800x600  600 cells, each with its background and a number changing every frame
//!   document  800x600  40 paragraphs (~20,000 chars) wrapped to a width changing every frame
//!
//! and the editor's (`editor.rs`): `editor` types a letter and takes it out every frame at
//! the middle of 100,000 lines of code, `editor-scroll` scrolls them 8 px a frame.

mod editor;
mod recorder;

use std::rc::Rc;

use quadra::{
    Align, AnyView, Color, Direction, Div, Justify, Length, Signal, WindowOptions, column, div, each_frame,
    expect, memo, provide, row, scroll, signal, text, uniform_list,
};
use quadra_native_win::DirectWrite;
use quadra_platform::HeadlessPlatform;
use quadra_render::{NullRenderer, Renderer};
use recorder::{Recorder, SharedPhases, say};

const WORDS: [&str; 32] = [
    "window", "button", "layout", "render", "scroll", "update", "shadow", "cursor", "memory", "thread",
    "signal", "bitmap", "stroke", "column", "editor", "kernel", "packet", "vector", "syntax", "buffer",
    "format", "cipher", "socket", "module", "branch", "commit", "server", "client", "screen", "pointer",
    "display", "palette",
];

/// The text of a seed, the same in every framework's app.
fn words(seed: u32, n: usize) -> String {
    let mut x = seed;
    let mut next = || {
        x = x.wrapping_mul(1103515245).wrapping_add(12345);
        WORDS[((x >> 16) % WORDS.len() as u32) as usize]
    };
    (0..n).map(|_| next()).collect::<Vec<_>>().join(" ")
}

fn cell_value(cell: usize, frame: u32) -> f64 {
    ((cell * 7919 + frame as usize * 31) % 100_000) as f64 / 100.0
}

fn cell_color(value: f64) -> Color {
    Color::rgb(if value < 333.0 {
        0xf6c9c4
    } else if value < 666.0 {
        0xf4e8b8
    } else {
        0xc6e9c8
    })
}

fn document_width(frame: u32) -> f32 {
    (600.0 + 150.0 * (frame as f64 / 30.0).sin()).round() as f32
}

pub(crate) fn full() -> Div {
    div().width(Length::Fill).height(Length::Fill).bg(Color::WHITE)
}

/// Counts frames with the recorder and gives each frame of the run its number.
pub(crate) fn animate(mut frame: impl FnMut(u32) + 'static) {
    let (mut recorder, mut n) = (Recorder::new(expect::<SharedPhases>()), 0);
    each_frame(move || {
        let more = recorder.tick();
        if more {
            n += 1;
            frame(n);
        }
        more
    });
}

fn counter() -> AnyView {
    let n = signal(0);
    let button = |label: &str, change: fn(&mut i32)| {
        let label = text(label.to_string()).size(13.0);
        let face = div().padding_x(12.0).padding_y(4.0).radius(6.0).bg(Color::rgb(0xe8e8e8));
        face.child(label).on_click(move |_| n.update(change))
    };
    let (less, reset, more) =
        (button("−", |n| *n -= 1), button("Reset", |n| *n = 0), button("+", |n| *n += 1));
    let buttons = row().gap(8.0).child(less).child(reset).child(more);
    let number = text(move || n.get().to_string()).size(56.0).weight(700);
    let page = full().direction(Direction::Column).align_items(Align::Center).justify(Justify::Center);
    AnyView::new(page.gap(16.0).child(number).child(buttons))
}

fn list() -> AnyView {
    let y = signal(0.0f32);
    animate(move |n| y.set(n as f32 * 8.0));
    let entry = |i: Signal<usize>| {
        let name = text(move || format!("Item {}", i.get())).size(13.0);
        let detail = text(move || words(i.get() as u32 + 1, 6)).size(11.0).color(Color::rgb(0x707070));
        let value = text(move || format!("{:.2}", cell_value(i.get(), 0))).size(13.0);
        let names = column().grow(1.0).child(name).child(detail);
        let line = row().width(Length::Fill).height(40.0).padding_x(12.0).gap(12.0);
        line.align_items(Align::Center).child(names).child(value)
    };
    AnyView::new(uniform_list(|| 100_000, 40.0, y, entry).container(full()))
}

fn grid() -> AnyView {
    let frame = signal(0u32);
    animate(move |n| frame.set(n));
    let cell = move |i: usize| {
        let value = memo(move || cell_value(i, frame.get()));
        let number = text(move || format!("{:.2}", value.get())).size(11.0).color(Color::rgb(0x101010));
        let centred = div().size(40.0, 20.0).align_items(Align::Center).justify(Justify::Center);
        centred.bg(move || cell_color(value.get())).child(number)
    };
    let line = move |r: usize| row().children((0..20).map(move |c| cell(r * 20 + c)));
    AnyView::new(full().direction(Direction::Column).children((0..30).map(line)))
}

fn document() -> AnyView {
    let frame = signal(0u32);
    animate(move |n| frame.set(n));
    let paragraph = |i: u32| div().child(text(words(i + 1, 70)).size(13.0));
    let page = column().width(move || document_width(frame.get())).padding(12.0).gap(8.0);
    AnyView::new(scroll(signal(0.0), page.children((0..40).map(paragraph))).container(full()))
}

fn main() -> quadra::Result<()> {
    let scene = std::env::args().nth(1).unwrap_or_else(|| "counter".into());
    let (view, (width, height)): (fn() -> AnyView, _) = match scene.as_str() {
        "counter" => (counter, (360.0, 260.0)),
        "list" => (list, (800.0, 600.0)),
        "grid" => (grid, (800.0, 600.0)),
        "document" => (document, (800.0, 600.0)),
        "editor" => (editor::typing, (800.0, 600.0)),
        "editor-scroll" => (editor::scrolling, (800.0, 600.0)),
        _ => {
            eprintln!("no scene {scene}");
            std::process::exit(2);
        }
    };
    let options = WindowOptions::new("Benchmark").size(width, height);
    if std::env::var_os("BENCH_HEADLESS").is_some() {
        // The passes alone, as fast as they go: real text, but no window and no GPU. For
        // finding where a frame's time goes; the numbers that count are the runner's.
        let text = DirectWrite::new().map_err(quadra::AppError::Text)?;
        let app =
            quadra_app::App::with(Rc::new(HeadlessPlatform::new()), NullRenderer::new(), Box::new(text));
        return run(app, options, view, true);
    }
    run(quadra::app()?, options, view, scene != "counter")
}

fn run<R: Renderer>(
    app: quadra_app::App<R>,
    options: WindowOptions,
    view: fn() -> AnyView,
    mut said: bool,
) -> quadra::Result<()> {
    let phases = SharedPhases::default();
    let seen = phases.clone();
    let app = app.on_presented(move |report| {
        seen.borrow_mut().add(report);
        if !std::mem::replace(&mut said, true) {
            say("ready"); // the counter's first frame is on its way to the screen
        }
    });
    let view = move || {
        provide(phases); // where the scene's recorder finds the frame reports
        view()
    };
    app.window(options, view)?.run()
}
