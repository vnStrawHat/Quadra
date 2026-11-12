//! Documents that are not ours (specs/12, "Fuzzing"): nothing a text can hold makes the
//! parsers fail, hang or recurse without bound. The corpus is that of the V tree.

use std::time::Duration;

use quadra_md::{Kind, anchor, block_at, outline, parse, parse_inline, plain, words};
use quadra_testing::fuzz::{Rng, mutate, run};

const CORPUS: [&str; 10] = [
    "# Title\n\nSome *text*\ncontinued.\n\n- one\n- two\n  - nested\n1. first\n",
    "```v\nfn main() {}\n```\n> quoted\n> more\n\n---\n| A | B |\n|---|:-:|\n| 1 | 2 |\n",
    "---\ntitle: x\n---\n## H2 ##\n- [ ] open\n- [x] done\n3) third\n",
    "a **b *c*** `d*e` [f **g**](http://x) snake_case_name <https://y> ~~s~~ ![img](a.png \"t\")",
    "$$\n\\frac{a}{b}\n$$\ninline $x^2$ and \\$5 and $$ y $$\n```math\n\\sqrt{2}\n```\n",
    "```mermaid\ngraph TD\nA-->B\n```\n<!-- comment\nmore -->\n<b>html</b> &amp; text\n",
    ">>> deep\n> > spaced\n\t- tab item\n    code?\n***\n___\n| x |\n|:--|\n",
    "```python\ndef f(x):\n    return \"s\" # c\n```\n~~~js\nlet a = `t${1}`; /* c */\n~~~\n",
    "[link](<http://a b>) [ref][1] [^note] http://bare.example/x?y=1. *a _b* c_ **unclosed",
    "Xin chào **thế giới** 漢字 😀 é\r\n\r\n1. a\r\n2. b\r\n",
];

/// The worst of their kind: nesting, what is never closed, tables without end.
fn deep(rng: &mut Rng) -> String {
    let n = 200 + rng.below(3000);
    match rng.below(10) {
        0 => format!("{} q", ">".repeat(n)),
        1 => (0..n / 10).map(|k| format!("{}- item\n", " ".repeat(k * 2))).collect(),
        2 => format!("{}x{}(y)", "[".repeat(n), "]".repeat(rng.below(n))),
        3 => format!("{}a{}", "*".repeat(n), "_".repeat(n)),
        4 => format!("```\n{}", "unclosed\n".repeat(n / 10)),
        5 => format!("$$\n{}", "x+".repeat(n)),
        6 => {
            let columns = 1 + rng.below(200);
            let line = |cell: &str| format!("|{}\n", cell.repeat(columns));
            format!("{}{}{}", line("h|"), line("-|"), line("c|").repeat(n / 20))
        }
        7 => format!("{}x{}", "[".repeat(n), "](u)".repeat(n)),
        8 => format!("{}{}", "[a](".repeat(n), "<https://b".repeat(n)),
        _ => format!("{}{}{}", "`".repeat(n), "$".repeat(n), "<!--".repeat(n / 4)),
    }
}

fn check(source: &str) {
    let blocks = parse(source);
    for block in &blocks {
        match &block.kind {
            Kind::Table { rows, .. } => rows.iter().flatten().for_each(|cell| drop(parse_inline(cell))),
            _ => drop(block.inline().map(parse_inline)),
        }
    }
    let _ = (outline(&blocks), block_at(&blocks, source.len() / 40), words(source), anchor(&plain(source)));
}

#[test]
fn nothing_a_document_can_hold_makes_the_parsers_fail_or_hang() {
    CORPUS.iter().for_each(|source| check(source));
    let corpus = CORPUS.map(str::as_bytes);
    let make = |rng: &mut Rng, _: usize| {
        if rng.chance(10) {
            return deep(rng).into_bytes();
        }
        let from = *rng.pick(&corpus);
        mutate(rng, from, &corpus)
    };
    run("markdown", 3000, Duration::from_secs(2), make, |input| check(&String::from_utf8_lossy(input)));
}

#[test]
fn a_megabyte_of_what_is_never_closed_is_read_in_the_time_its_length_takes() {
    // Each of these would be searched for from every place to the end, were a search that
    // failed not remembered.
    for unit in ["[", "[a](", "`a ", "<!-- ", "<https://a ", "[[a](b)", "$a ", "**_"] {
        let text = unit.repeat(1_000_000 / unit.len());
        let started = std::time::Instant::now();
        let spans = parse_inline(&text);
        assert!(started.elapsed() < Duration::from_secs(2), "{unit:?} took {:?}", started.elapsed());
        assert!(!spans.is_empty());
    }
}
