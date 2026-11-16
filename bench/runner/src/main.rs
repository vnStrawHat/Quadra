//! The comparison bench's runner (docs/benchmarks.md, "Method"), Windows only. It starts each
//! app with a scene, reads what it says on standard output and measures its whole process from
//! outside: processor time per frame, peak private working set, idle memory and processor
//! time, startup.
//!
//! `run [-runs 5] [-out file.json] name=app.exe [name=app.exe ...]`
//!
//! Each round's measures go to standard error, a table of the medians to standard output, and
//! the medians with their lows and highs as JSON to the file. Without one the JSON goes to
//! standard output and the table to standard error.

mod scene;

use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::Duration;

use scene::Measures;

const ANIMATED: [&str; 3] = ["list", "grid", "document"];
const FRAMES: usize = 600;
const IDLE: Duration = Duration::from_secs(5);

/// Every value each measure of each scene took, in the order they came.
type Samples = Vec<(String, Vec<(&'static str, Vec<f64>)>)>;

fn add(samples: &mut Samples, scene: &str, measures: Measures) {
    let at = match samples.iter().position(|(s, _)| s == scene) {
        Some(at) => at,
        None => {
            samples.push((scene.to_string(), Vec::new()));
            samples.len() - 1
        }
    };
    let of = &mut samples[at].1;
    for (name, value) in measures {
        match of.iter_mut().find(|(n, _)| *n == name) {
            Some((_, values)) => values.push(value),
            None => of.push((name, vec![value])),
        }
    }
}

/// The median, lowest and highest.
fn series(values: &[f64]) -> (f64, f64, f64) {
    let mut s = values.to_vec();
    s.sort_by(f64::total_cmp);
    let n = s.len();
    let median = if n.is_multiple_of(2) { (s[n / 2 - 1] + s[n / 2]) / 2.0 } else { s[n / 2] };
    (median, s[0], s[n - 1])
}

fn fail(message: String) -> ! {
    eprintln!("run: {message}");
    std::process::exit(1);
}

fn show(measures: &Measures) -> String {
    measures.iter().map(|(name, value)| format!("{name} {value:.4}")).collect::<Vec<_>>().join(", ")
}

fn main() {
    let (mut runs, mut out, mut apps) = (5usize, None, Vec::<(String, PathBuf)>::new());
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-runs" => {
                runs = args.next().and_then(|n| n.parse().ok()).unwrap_or_else(|| fail("-runs N".into()))
            }
            "-out" => out = Some(args.next().unwrap_or_else(|| fail("-out FILE".into()))),
            _ => match arg.split_once('=') {
                Some((name, path)) => apps.push((name.to_string(), PathBuf::from(path))),
                None => fail(format!("not name=app.exe: {arg}")),
            },
        }
    }
    if apps.is_empty() || runs == 0 {
        fail("usage: run [-runs 5] [-out file.json] name=app.exe [name=app.exe ...]".into());
    }
    for (name, path) in &apps {
        if !path.is_file() {
            fail(format!("{name}: no file {}", path.display()));
        }
    }
    // Measured once before anything else, so its 200 ms fall in no measure.
    let rate = quadra_shell::cycles_per_second();
    eprintln!("cycle counter: {:.0} MHz", rate / 1e6);
    let mut samples: Vec<Samples> = apps.iter().map(|_| Vec::new()).collect();
    for (name, path) in &apps {
        // One start that is not measured, so the first measured one is no cold read of the disk.
        scene::counter(path, Duration::ZERO).unwrap_or_else(|e| fail(format!("{name}: {e}")));
    }
    for round in 1..=runs {
        for ((name, path), samples) in apps.iter().zip(&mut samples) {
            let m = scene::counter(path, IDLE).unwrap_or_else(|e| fail(format!("{name} counter: {e}")));
            eprintln!("round {round} {name} counter: {}", show(&m));
            add(samples, "counter", m);
        }
        for scene in ANIMATED {
            for ((name, path), samples) in apps.iter().zip(&mut samples) {
                let m = scene::animated(path, scene, FRAMES)
                    .unwrap_or_else(|e| fail(format!("{name} {scene}: {e}")));
                eprintln!("round {round} {name} {scene}: {}", show(&m));
                add(samples, scene, m);
            }
        }
    }
    let (table, json) = (table(&apps, &samples), json(&apps, &samples, runs));
    match out {
        Some(file) => {
            print!("{table}");
            std::fs::write(&file, json).unwrap_or_else(|e| fail(format!("{file}: {e}")));
        }
        // Standard output is the JSON alone then, as the first runner's was.
        None => {
            eprint!("{table}");
            print!("{json}");
        }
    }
}

/// The medians, a row a measure and a column an app; memory in megabytes.
fn table(apps: &[(String, PathBuf)], samples: &[Samples]) -> String {
    let mut t = format!("{:<10}{:<18}", "scene", "measure");
    for (name, _) in apps {
        let _ = write!(t, "{name:>14}");
    }
    t.push('\n');
    for (scene, measures) in &samples[0] {
        for (i, (measure, _)) in measures.iter().enumerate() {
            let _ = write!(t, "{:<10}{measure:<18}", if i == 0 { scene.as_str() } else { "" });
            for of in samples {
                let values = of
                    .iter()
                    .find(|(s, _)| s == scene)
                    .and_then(|(_, m)| m.iter().find(|(n, _)| n == measure));
                let median = values.map_or(f64::NAN, |(_, v)| series(v).0);
                let _ = match measure.ends_with("Memory") {
                    true => write!(t, "{:>11.1} MB", median / 1e6),
                    false => write!(t, "{median:>14.3}"),
                };
            }
            t.push('\n');
        }
    }
    let _ = write!(t, "{:<28}", "binary size");
    for (_, path) in apps {
        let _ = write!(t, "{:>11.2} MB", size(path) as f64 / 1e6);
    }
    t.push('\n');
    t
}

fn size(path: &std::path::Path) -> u64 {
    std::fs::metadata(path).map_or(0, |m| m.len())
}

fn quoted(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// What the V runner wrote, in its order: the date, the machine, the rounds and frames, then
/// each app's binary size and, for each scene, each measure's median, low and high.
fn json(apps: &[(String, PathBuf)], samples: &[Samples], runs: usize) -> String {
    let now = quadra_app::time::DateTime::from_unix(quadra_app::time::unix_now(), 0);
    let (d, t) = (now.date, now.time);
    let date = format!("{d}T{:02}:{:02}:{:02}Z", t.hour, t.minute, t.second);
    let mut j = format!(
        "{{\n\t\"date\": {},\n\t\"machine\": {},\n\t\"runs\": {runs},\n\t\"frames\": {FRAMES},\n\t\"frameworks\": {{",
        quoted(&date),
        quoted(&machine())
    );
    for (a, ((name, path), scenes)) in apps.iter().zip(samples).enumerate() {
        let _ = write!(
            j,
            "{}\n\t\t{}: {{\n\t\t\t\"size\": {},\n\t\t\t\"scenes\": {{",
            comma(a),
            quoted(name),
            size(path)
        );
        for (s, (scene, measures)) in scenes.iter().enumerate() {
            let _ = write!(j, "{}\n\t\t\t\t{}: {{", comma(s), quoted(scene));
            for (m, (measure, values)) in measures.iter().enumerate() {
                let (median, low, high) = series(values);
                let _ = write!(
                    j,
                    "{}\n\t\t\t\t\t\"{measure}\": {{ \"median\": {median}, \"low\": {low}, \"high\": {high} }}",
                    comma(m)
                );
            }
            j.push_str("\n\t\t\t\t}");
        }
        j.push_str("\n\t\t\t}\n\t\t}");
    }
    j.push_str("\n\t}\n}\n");
    j
}

fn comma(i: usize) -> &'static str {
    if i == 0 { "" } else { "," }
}

/// The processor's name, from the registry.
fn machine() -> String {
    let key = r"HKLM\HARDWARE\DESCRIPTION\System\CentralProcessor\0";
    let out = std::process::Command::new("reg").args(["query", key, "/v", "ProcessorNameString"]).output();
    let text = out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    text.lines()
        .find_map(|l| l.split_once("REG_SZ").map(|(_, name)| name.trim().to_string()))
        .unwrap_or_else(|| "unknown".into())
}
