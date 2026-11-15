//! The comparison bench's scenes without a window (specs/14, "Baselines"): what the passes
//! cost on the CPU, on the fake text backend and the null renderer.
//!
//!     tools/cargo.sh run --release -p quadra-view --example scenes
//!
//! `list` is a virtual list of 100,000 rows of three texts, scrolling 8 px a frame.
//! `grid` is 600 cells, each with its own background and a number that changes every frame.
//! `document` is 40 paragraphs, about 20,000 characters, wrapped to a width that changes
//! every frame. The numbers are not comparable with a real window's yet: no glyph is
//! rasterized and nothing reaches a GPU.

use std::time::Instant;

use quadra_prim::{Color, Size};
use quadra_reactive::signal;
use quadra_render::{DeviceSize, NullRenderer, Renderer, SurfaceTarget};
use quadra_text::FakeBackend;
use quadra_tree::{Fonts, FrameStats, Tree};
use quadra_view::{View, column, div, mount_root, row, text, uniform_list};

const FRAMES: u32 = 600;

fn run(name: &str, view: impl View, mut change: impl FnMut(u32)) {
    let fonts = Fonts::new(Box::new(FakeBackend::new()));
    let mut tree = Tree::new(fonts.clone());
    let mut renderer = NullRenderer::new();
    let surface =
        renderer.create_surface(SurfaceTarget::Offscreen, DeviceSize { width: 800, height: 600 }).unwrap();
    let size = Size::new(800.0, 600.0);

    let start = Instant::now();
    mount_root(&mut tree, view);
    tree.frame(size, 1.0);
    renderer.sync_atlas(fonts.borrow_mut().glyphs.core_mut()).unwrap();
    renderer.draw(surface, tree.scene_mut(), Color::WHITE).unwrap();
    let first = start.elapsed().as_secs_f64() * 1e3;

    for n in 0..60 {
        change(n); // warm up
        tree.frame(size, 1.0);
    }
    let mut total = FrameStats::default();
    let mut uploaded = 0;
    let start = Instant::now();
    for n in 0..FRAMES {
        change(100 + n);
        let stats = tree.frame(size, 1.0);
        renderer.sync_atlas(fonts.borrow_mut().glyphs.core_mut()).unwrap();
        renderer.draw(surface, tree.scene_mut(), Color::WHITE).unwrap();
        total.bindings_run += stats.bindings_run;
        total.nodes_laid_out += stats.nodes_laid_out;
        total.nodes_painted += stats.nodes_painted;
        total.instances_written += stats.instances_written;
        uploaded += renderer.stats().instances_uploaded;
    }
    let ms = start.elapsed().as_secs_f64() * 1e3 / FRAMES as f64;
    let per = |n: u32| n / FRAMES;
    println!(
        "{name:<9} first frame {first:6.2} ms; then {ms:.3} ms a frame: {} bindings, {} laid out, {} painted, {} instances written, {} uploaded",
        per(total.bindings_run),
        per(total.nodes_laid_out),
        per(total.nodes_painted),
        per(total.instances_written),
        per(uploaded),
    );
}

fn main() {
    let scroll = signal(0.0f32);
    let entry = |i: quadra_reactive::Signal<usize>| {
        let name = text(move || format!("Row number {}", i.get())).size(10.0);
        let detail = text(move || format!("detail {}", i.get() * 3)).size(10.0);
        let state = text(move || if i.get().is_multiple_of(2) { "ready" } else { "waiting" }).size(10.0);
        row().gap(12.0).child(name).child(detail).child(state)
    };
    run("list", uniform_list(|| 100_000, 28.0, scroll, entry), move |n| scroll.set(n as f32 * 8.0));

    let tick = signal(0u32);
    let cell = move |i: u32| {
        let number = text(move || (tick.get() * 7 + i * 13).to_string()).size(10.0);
        div().size(26.0, 28.0).bg(Color::rgb(0x202020 + i * 0x000301)).child(number)
    };
    let line = move |r: u32| row().children((0..30).map(move |c| cell(r * 30 + c)));
    run("grid", column().children((0..20).map(line)), move |n| tick.set(n));

    let width = signal(700.0f32);
    let words = "the quick brown fox jumps over the lazy dog and keeps running through the field ";
    let paragraph = move |_| text(words.repeat(6)).size(10.0); // about 480 characters
    let page = column().width(width).gap(8.0).children((0..40).map(paragraph));
    run("document", page, move |n| width.set(500.0 + (n % 200) as f32));
}
