//! Micro-bench of propagation (specs/14, Tooling): what one binding costs.
//!
//!     tools/cargo.sh run --release -p quadra-reactive --example propagation
//!
//! The shape is the comparison bench's `grid`, the worst case for fine-grained updates: every
//! cell changes every frame, so every binding runs every frame.

use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;
use std::time::Instant;

use quadra_reactive::{batch, deferred, memo, signal};

const CELLS: usize = 600;
const FRAMES: usize = 2_000;

fn per_binding(label: &str, mut frame: impl FnMut(u32)) {
    for n in 0..200 {
        frame(n); // warm up
    }
    let start = Instant::now();
    for n in 0..FRAMES as u32 {
        frame(1_000 + n);
    }
    let ns = start.elapsed().as_nanos() as f64 / (FRAMES * CELLS) as f64;
    println!(
        "{label:<44} {ns:>7.1} ns per binding, {:>6.1} us per frame of {CELLS}",
        ns * CELLS as f64 / 1e3
    );
}

fn main() {
    let sink = Rc::new(Cell::new(0u32));

    // One signal, one binding per cell: write all, then run all (a frame's update pass).
    let due = Rc::new(Cell::new(0u32));
    let cells: Vec<_> = (0..CELLS).map(|_| signal(0u32)).collect();
    let bindings: Vec<_> = cells
        .iter()
        .map(|&cell| {
            let (due, sink) = (due.clone(), sink.clone());
            deferred(move || due.set(due.get() + 1), move || sink.set(sink.get().wrapping_add(cell.get())))
        })
        .collect();
    per_binding("signal -> binding", |n| {
        batch(|| cells.iter().for_each(|c| c.set(n)));
        bindings.iter().for_each(|b| b.run());
    });

    // The same through a memo per cell.
    let cells: Vec<_> = (0..CELLS).map(|_| signal(0u32)).collect();
    let bindings: Vec<_> = cells
        .iter()
        .map(|&cell| {
            let double = memo(move || cell.get() * 2);
            let sink = sink.clone();
            deferred(|| {}, move || sink.set(sink.get().wrapping_add(double.get())))
        })
        .collect();
    per_binding("signal -> memo -> binding", |n| {
        batch(|| cells.iter().for_each(|c| c.set(n)));
        bindings.iter().for_each(|b| b.run());
    });

    // One cell of the 600 changes: the cost must not depend on the other 599.
    let cells: Vec<_> = (0..CELLS).map(|_| signal(0u32)).collect();
    let bindings: Vec<_> = cells
        .iter()
        .map(|&cell| {
            let sink = sink.clone();
            deferred(|| {}, move || sink.set(sink.get().wrapping_add(cell.get())))
        })
        .collect();
    let start = Instant::now();
    for n in 0..(FRAMES * CELLS) as u32 {
        cells[7].set(n + 1);
        bindings[7].run();
    }
    let ns = start.elapsed().as_nanos() as f64 / (FRAMES * CELLS) as f64;
    println!("{:<44} {ns:>7.1} ns per change", "one cell of 600 changes");
    black_box((sink.get(), due.get()));
}
