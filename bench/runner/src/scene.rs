//! One start of an app with a scene, measured from outside: the protocol is the app's lines on
//! standard output (`ready`, `start`, `done {...}`; `bench/compare/src/recorder.rs`).

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use quadra_shell::{cycles_per_second, private_memory, process_cycles, window_shown};

/// What one start measured, by name, in the order the results keep.
pub type Measures = Vec<(&'static str, f64)>;

/// A started app: killed when dropped, so a failed run leaves no window behind.
struct App {
    child: Child,
    lines: Receiver<String>,
}

/// The processor time of the app's threads in seconds, and its private working set.
struct Reading {
    cpu: f64,
    memory: u64,
}

impl App {
    fn start(path: &Path, scene: &str) -> Result<App, String> {
        let mut child = Command::new(path)
            .arg(scene)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("cannot start {}: {e}", path.display()))?;
        let out = child.stdout.take().expect("piped");
        let (send, lines) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        Ok(App { child, lines })
    }

    /// Reads standard output until a line that starts with `prefix` (3 minutes at most).
    ///
    /// It spins rather than blocks, on purpose: the method's numbers were taken by a runner
    /// that kept one core busy while it waited, which holds the processor's clock up, and a
    /// frame that runs at a lower clock takes up to twice the cycles. Blocking here would
    /// make every number since incomparable (docs/benchmarks.md, "Method").
    fn wait_line(&self, prefix: &str) -> Result<String, String> {
        let deadline = Instant::now() + Duration::from_secs(180);
        while Instant::now() < deadline {
            match self.lines.try_recv() {
                Ok(line) if line.trim().starts_with(prefix) => return Ok(line.trim().to_string()),
                Ok(_) | Err(TryRecvError::Empty) => std::hint::spin_loop(),
                Err(TryRecvError::Disconnected) => return Err(format!("exited before saying \"{prefix}\"")),
            }
        }
        Err(format!("did not say \"{prefix}\""))
    }

    fn reading(&self) -> Reading {
        let cycles = process_cycles(&self.child).unwrap_or(0);
        Reading { cpu: cycles as f64 / cycles_per_second(), memory: private_memory(&self.child).unwrap_or(0) }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The counter: startup (the later of its first frame and its window on screen, from before
/// it was started), then its memory and processor time at rest, `idle` to twice `idle` after.
pub fn counter(path: &Path, idle: Duration) -> Result<Measures, String> {
    let began = Instant::now();
    let app = App::start(path, "counter")?;
    let (drawn, on_screen) = thread::scope(|s| {
        let child = &app.child;
        let shown = s.spawn(move || {
            while began.elapsed() < Duration::from_secs(60) {
                if window_shown(child) {
                    return Some(began.elapsed());
                }
                thread::sleep(Duration::from_millis(1));
            }
            None
        });
        let drawn = app.wait_line("ready").map(|_| began.elapsed());
        (drawn, shown.join().expect("the watch of the window"))
    });
    let startup = drawn?.max(on_screen.ok_or("no window came on screen")?);
    thread::sleep(idle);
    let before = app.reading();
    thread::sleep(idle);
    let after = app.reading();
    Ok(vec![
        ("startupMs", startup.as_secs_f64() * 1e3),
        ("idleMemory", after.memory as f64),
        ("idleCpuMsPerSec", (after.cpu - before.cpu) * 1e3 / idle.as_secs_f64()),
    ])
}

/// A scene that draws every frame: processor time over the measured `frames`, the frame
/// intervals the app says, and the peak of its private working set, sampled every 10 ms.
pub fn animated(path: &Path, scene: &str, frames: usize) -> Result<Measures, String> {
    let app = App::start(path, scene)?;
    let stopped = AtomicBool::new(false);
    let (run, peak) = thread::scope(|s| {
        let (child, stopped) = (&app.child, &stopped);
        let sampler = s.spawn(move || {
            let mut peak = 0;
            while !stopped.load(Ordering::Relaxed) {
                peak = peak.max(private_memory(child).unwrap_or(0));
                thread::sleep(Duration::from_millis(10));
            }
            peak
        });
        let run = (|| {
            app.wait_line("start")?;
            let before = app.reading();
            let line = app.wait_line("done")?;
            Ok::<_, String>((before, app.reading(), line))
        })();
        stopped.store(true, Ordering::Relaxed);
        (run, sampler.join().expect("the sampler"))
    });
    let (before, after, line) = run?;
    let field = |key| field(&line, key).ok_or_else(|| format!("no \"{key}\" in: {line}"));
    let (count, wall) = (field("frames")?, field("wallUs")?);
    Ok(vec![
        ("cpuMsPerFrame", (after.cpu - before.cpu) * 1e3 / frames as f64),
        ("fps", count / (wall / 1e6)),
        ("p50Ms", field("p50Us")? / 1e3),
        ("p99Ms", field("p99Us")? / 1e3),
        ("maxMs", field("maxUs")? / 1e3),
        ("peakMemory", peak as f64),
    ])
}

/// The number of a key of the flat object of the `done` line.
fn field(line: &str, key: &str) -> Option<f64> {
    let at = line.find(&format!("\"{key}\":"))? + key.len() + 3;
    let rest = line[at..].trim_start();
    let end = rest.find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | 'e' | 'E' | '+')))?;
    rest[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_done_line_is_read() {
        let line = r#"done {"frames":600,"wallUs":8001234,"p50Us":13334,"p99Us":13445,"maxUs":13506}"#;
        assert_eq!(super::field(line, "frames"), Some(600.0));
        assert_eq!(super::field(line, "maxUs"), Some(13506.0));
        assert_eq!(super::field(line, "fps"), None);
    }
}
