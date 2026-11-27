//! Reading another process from outside (specs/16, "Process usage"): the test binary is run
//! again as a child that spins and holds memory, and is read while it waits to be killed.
#![cfg(any(windows, target_os = "macos"))]

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const CHILD: &str = "QUADRA_MEASURE_CHILD";
const HELD: usize = 64 << 20;

/// The child's part: nothing when the test runs as itself.
#[test]
fn child() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    let held = vec![1u8; HELD];
    let began = Instant::now();
    while began.elapsed() < Duration::from_millis(300) {}
    println!("spun {}", held.iter().map(|&b| b as usize).sum::<usize>());
    let _ = std::io::stdout().flush();
    std::thread::sleep(Duration::from_secs(60));
}

#[test]
fn a_child_is_read_from_outside() {
    let rate = quadra_shell::cycles_per_second();
    assert!((1e8..1e11).contains(&rate), "a cycle counter of {rate} a second");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child", "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .expect("the test binary runs again");
    let out = BufReader::new(child.stdout.take().unwrap());
    let spun = out.lines().map_while(Result::ok).any(|line| line.contains("spun"));
    let (cycles, memory, shown) = (
        quadra_shell::process_cycles(&child),
        quadra_shell::private_memory(&child),
        quadra_shell::window_shown(&child),
    );
    let _ = child.kill();
    let _ = child.wait();
    assert!(spun, "the child said it spun");
    let cpu = cycles.expect("the system reads the cycles") as f64 / rate;
    assert!((0.25..2.0).contains(&cpu), "300 ms of spinning took {cpu} s of the processor");
    let memory = memory.expect("the system reads the memory");
    assert!(memory as usize > HELD, "a private working set of {memory} bytes holds {HELD}");
    assert!(!shown, "the child has no window");
}
