//! The fire demo's frames fed straight to the terminal engine `quadra-term` uses, with no
//! console and no window (specs/14, "Terminal throughput"): the engine's ceiling. As in a
//! session, the engine is behind a lock that another thread takes 60 times a second for the
//! snapshot a frame is drawn from.
//!
//!     doomfire-feed [--seconds N] [--out FILE] [--size COLSxROWS]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use quadra_bench_doomfire::{Fire, START, Sizes, args, summary};
use quadra_term::vt::{Config, EventBatch, Size, SnapshotState, Terminal};

fn main() {
    let args = args();
    let (cols, rows) = args.size.unwrap_or((200, 50));
    let term = Arc::new(Mutex::new(Terminal::new(Size { rows, cols }, Config::default())));
    let (done, drawn) = (Arc::new(AtomicBool::new(false)), term.clone());
    let ends = done.clone();
    let ui = std::thread::spawn(move || {
        let (mut state, mut frames, mut held) = (SnapshotState::new(), 0u32, Duration::ZERO);
        while !ends.load(Ordering::Relaxed) {
            let mut term = drawn.lock().unwrap();
            let at = Instant::now();
            term.snapshot_update(&mut state, at);
            held += at.elapsed();
            drop(term);
            frames += 1;
            std::thread::sleep(Duration::from_millis(16));
        }
        (frames, held / frames.max(1))
    });
    let mut batch = EventBatch::new();
    let mut feed = |bytes: &[u8]| {
        let mut term = term.lock().unwrap();
        term.feed(bytes, &mut batch, Instant::now());
        batch.clear();
    };
    feed(START);
    let mut fire = Fire::new(cols, rows, 0x9e37_79b9_7f4a_7c15);
    let (mut frame, mut sizes) = (Vec::with_capacity(256 * 1024), Sizes::default());
    let (start, end) = (Instant::now(), args.seconds.unwrap_or(5.0));
    while start.elapsed().as_secs_f64() < end {
        fire.spread();
        fire.frame(&mut frame);
        // In reads of 16 KB, as the pseudo console's reader takes them.
        for part in frame.chunks(16 * 1024) {
            feed(part);
        }
        sizes.add(frame.len() as u64);
        let fps = sizes.frames as f64 / start.elapsed().as_secs_f64();
        feed(&fire.status(&sizes, fps));
    }
    let said = summary(cols, rows, &sizes, start);
    done.store(true, Ordering::Relaxed);
    let (snapshots, each) = ui.join().unwrap_or_default();
    println!("doomfire-feed: {said} ({snapshots} snapshots, {} µs each)", each.as_micros());
    if let Some(out) = args.out {
        let _ = std::fs::write(out, said);
    }
}
