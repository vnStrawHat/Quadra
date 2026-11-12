//! Diagrams come from documents nobody vouches for: no source may make the parser or the
//! layout fail, hang, or work without bound (specs/12, "Fuzzing").

use std::time::{Duration, Instant};

use quadra_testing::fuzz::{Rng, mutate, run};
use quadra_text::{FakeBackend, TextStyle, TextSystem};

use super::draw::Measure;
use super::flow::{LONGEST_LINK, MOST_SUBGRAPHS};
use super::{Chart, read};

const CORPUS: [&str; 5] = [
    "graph TD\nA[Start] --> B{Is it?}\nB -->|Yes| C[OK]\nC --> D[Rethink]\nD --> B\nB ---->|No| E[End]\nsubgraph work [Work]\nC\nD\nend\nF((Hub)) --> A & C & E",
    "flowchart LR\nA --> B\nB --- C\nC -.-> D\nD ==> E\nE --o F\nF --x G\nG <--> H\nH --->|long| I\nI -- text --> J\nJ -. dots .-> K",
    "graph TD\na[Rect]\nb(Round)\nc([Stadium])\nd((Circle))\ne{Diamond}\nf{{Hex}}\ng[[Sub]]\nh[(DB)]\ni>Flag]\nj[\"Quoted [x]\"]\nk:::cls\nclassDef cls fill:#f00\n%% comment",
    "---\ntitle: x\n---\nflowchart BT\nsubgraph a\nsubgraph b [B]\nx --> y\nend\nend\ny --> x; x --> x",
    "graph\nA-->B-->C-->A\nA & B & C --> D & E & F\n漢字 --> 😀[\"Xin chào\"]",
];

/// What is deep, long or wide, which a change of a byte does not make.
fn deep(rng: &mut Rng) -> String {
    let n = 50 + rng.below(600);
    match rng.below(4) {
        0 => format!("graph TD\n{}a --> b\n{}", "subgraph s\n".repeat(n % 150), "end\n".repeat(rng.below(n))),
        1 => format!("graph LR\na {}> b", "-".repeat(n)),
        // Many nodes, in a chain or in cycles.
        2 => (0..n).fold("flowchart TD".to_string(), |all, k| {
            format!("{all}\nn{k} --> n{}", (k + 1 + rng.below(3)) % n)
        }),
        // Wide groups: every pair is a link.
        _ => {
            let group = (0..1 + n / 10).map(|k| format!("v{k}")).collect::<Vec<_>>().join(" & ");
            format!("graph TD\n{group} --> {group}")
        }
    }
}

/// Reads a source and, when it is a diagram, lays it out.
fn check(source: &str, ts: &mut TextSystem) -> Result<Chart, String> {
    let chart = read(source)?;
    let drawing = chart.layout(&mut Measure { ts, base: TextStyle { size: 14.0, ..TextStyle::default() } });
    assert!(drawing.width.is_finite() && drawing.height.is_finite(), "{source}");
    Ok(chart)
}

#[test]
fn nothing_a_diagram_can_hold_makes_it_fail_hang_or_work_without_bound() {
    let mut ts = TextSystem::new(Box::new(FakeBackend::new()));
    for source in CORPUS {
        assert!(check(source, &mut ts).is_ok(), "{source}");
    }
    // What costs too much is a fault before anything is laid out: every pair of two wide
    // groups, subgraphs without end, links back across a long chain.
    let group = (0..100).map(|k| format!("v{k}")).collect::<Vec<_>>().join(" & ");
    assert!(check(&format!("graph TD\n{group} --> {group}"), &mut ts).is_err());
    assert!(check(&format!("graph TD\n{}a", "subgraph s\n".repeat(MOST_SUBGRAPHS + 1)), &mut ts).is_err());
    let cycle: String = (1..600).map(|k| format!("n{} --> n{k}\nn{k} --> n0\n", k - 1)).collect();
    assert!(check(&format!("graph TD\n{cycle}"), &mut ts).is_err());
    // A link as long as a megabyte asks for as many ranks as any link may, and is read in
    // the time its length takes.
    let started = Instant::now();
    let Ok(Chart::Flow(long)) = check(&format!("graph LR\na {}> b", "-".repeat(1_000_000)), &mut ts) else {
        panic!()
    };
    assert_eq!(long.edges[0].span, LONGEST_LINK);
    // Links with their labels in them, then one that never closes: what is not there is
    // looked for once, not once a link.
    let never = format!("graph TD\na{} -- {}", " -- t --> a".repeat(400), "z".repeat(1_000_000));
    assert!(check(&never, &mut ts).is_err());
    assert!(started.elapsed() < Duration::from_secs(2), "{:?}", started.elapsed());
    // A node said again in a group is one node: not links side by side.
    let Ok(Chart::Flow(again)) = check(&format!("graph\nA & {}C --> D & E & F", "B& ".repeat(500)), &mut ts)
    else {
        panic!()
    };
    assert_eq!(again.edges.len(), 9);

    let corpus: Vec<&[u8]> = CORPUS.iter().map(|source| source.as_bytes()).collect();
    let make = |rng: &mut Rng, _: usize| {
        let from = *rng.pick(&corpus);
        if rng.chance(8) { deep(rng).into_bytes() } else { mutate(rng, from, &corpus) }
    };
    run("mermaid", 1000, Duration::from_secs(2), make, |bytes| {
        let _ = check(&String::from_utf8_lossy(bytes), &mut ts);
    });
}
