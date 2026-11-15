//! The fire demo in the console it runs in (specs/14, "Terminal throughput"): a frame a write,
//! then the status line with the fps so far.
//!
//!     doomfire [--seconds N] [--out FILE] [--size COLSxROWS]
//!
//! Without `--seconds` it burns until Ctrl+C. With it, it ends after N seconds, says the
//! summary and writes it to FILE.

use std::time::Instant;

use quadra_bench_doomfire::{END, Fire, START, Sizes, args, summary};
use quadra_shell::console;

fn main() {
    let args = args();
    console::enable_vt();
    let (cols, rows) = args.size.or_else(console::size).unwrap_or((80, 24));
    console::write(START);
    let mut fire = Fire::new(cols, rows, 0x9e37_79b9_7f4a_7c15);
    let (mut frame, mut sizes) = (Vec::with_capacity(256 * 1024), Sizes::default());
    let start = Instant::now();
    loop {
        fire.spread();
        fire.frame(&mut frame);
        console::write(&frame);
        sizes.add(frame.len() as u64);
        let seconds = start.elapsed().as_secs_f64();
        console::write(&fire.status(&sizes, sizes.frames as f64 / seconds));
        if args.seconds.is_some_and(|end| seconds >= end) {
            break;
        }
    }
    let said = summary(cols, rows, &sizes, start);
    console::write(END);
    println!("doomfire: {said}");
    if let Some(out) = args.out {
        let _ = std::fs::write(out, said);
    }
}
