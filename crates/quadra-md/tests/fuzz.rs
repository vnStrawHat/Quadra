//! Documents that are not ours (specs/12, "Fuzzing"): nothing a text can hold makes the
//! parsers fail, hang or recurse without bound. The corpus is that of the V tree.

use std::time::Duration;

use quadra_md::{Kind, anchor, block_at, outline, parse, parse_inline, plain, tex, words};
use quadra_testing::fuzz::{Rng, mutate, run};
use quadra_text::{FakeBackend, TextSystem};

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

const FORMULAS: [&str; 9] = [
    r"\frac{x}{y} + x^2_i - \sqrt[3]{x} \cdot \sum_{i=0}^{n} i",
    r"\left( \frac{\frac{a}{b}}{c} \right] \big| \Big\{ x \bigg\}",
    r"\begin{pmatrix} a & b \\ c & d \end{pmatrix} \begin{array}{cc} 1 & 2 \\[4pt] 3 & 4 \end{array}",
    r"\begin{cases} x & x > 0 \\ -x & \text{otherwise} \end{cases} \operatorname*{arg\,max}_x f",
    r"f'(x) = \lim\limits_{h \to 0} \frac{f(x+h) - f(x)}{h} \int_0^\infty e^{-x}\,dx f'''",
    r"\mathbb{R} \mathcal{F} \mathbf{v} \hat{a} \vec{b} \overline{xy} \underbrace{abc}_{n}",
    "\\alpha\\beta\\gamma \\le \\ge \\ne \\pm \\infty % comment\n\\binom{n}{k} \\tfrac12 \\color{red}{x} \\mathrm{d}x",
    r"\begin{aligned} a &= b + c \\ &= d \end{aligned} \middle| \phantom{x} \quad \qquad",
    "αβγ 漢 😀 \\text{Xin chào} é",
];

/// What opens and is never closed.
const OPENERS: [&str; 10] = [
    "{",
    r"\frac{",
    "x^{",
    r"\sqrt{",
    r"\left(",
    r"\begin{matrix}",
    r"\sqrt[",
    r"\text{",
    r"\overline{",
    "a_{",
];

/// How deep the nodes of a formula nest.
fn depth(nodes: &[tex::Node]) -> usize {
    use tex::Node::*;
    let of = |node: &tex::Node| match node {
        Group(body) | Delim { body, .. } | Accent { body, .. } | Line { body, .. } => depth(body),
        Frac { over, under, .. } => depth(over).max(depth(under)),
        Sqrt { index, body } => depth(body).max(index.as_deref().map_or(0, depth)),
        Scripts { base, sup, sub } => depth(std::slice::from_ref(base))
            .max(sup.as_deref().map_or(0, depth))
            .max(sub.as_deref().map_or(0, depth)),
        Stack { body, over, under, .. } => {
            depth(body).max(over.as_deref().map_or(0, depth)).max(under.as_deref().map_or(0, depth))
        }
        Matrix { rows, .. } => rows.iter().flatten().map(|cell| depth(cell)).max().unwrap_or(0),
        _ => 0,
    };
    1 + nodes.iter().map(of).max().unwrap_or(0)
}

#[test]
fn nothing_a_formula_can_hold_makes_its_parser_fail_hang_or_nest_without_bound() {
    for opener in OPENERS {
        let started = std::time::Instant::now();
        let (nodes, fault) = tex::parse(&opener.repeat(20_000));
        assert!(fault.is_some(), "{opener}");
        assert!(depth(&nodes) <= tex::DEEPEST, "{opener}");
        assert!(started.elapsed() < Duration::from_secs(2), "{opener:?} took {:?}", started.elapsed());
    }
    let corpus = FORMULAS.map(str::as_bytes);
    let make = |rng: &mut Rng, _: usize| {
        let n = 50 + rng.below(5000);
        match rng.below(20) {
            0 => {
                let row = format!("{}\\\\ ", "a & ".repeat(1 + rng.below(60)));
                format!(
                    "\\begin{{matrix}}{}{}",
                    row.repeat(n / 50),
                    if rng.chance(50) { "\\end{matrix}" } else { "" }
                )
                .into_bytes()
            }
            1 => format!("{}x{}", rng.pick(&OPENERS).repeat(n), "}".repeat(rng.below(n))).into_bytes(),
            _ => {
                let from = *rng.pick(&corpus);
                mutate(rng, from, &corpus)
            }
        }
    };
    // What is read is laid out too, in display and in a line: a formula with a fault shows
    // its source, but nothing says the nodes of one cannot be laid out.
    let mut ts = TextSystem::new(Box::new(FakeBackend::new()));
    let mut check = |input: &[u8]| {
        let (nodes, _) = tex::parse(&String::from_utf8_lossy(input));
        assert!(depth(&nodes) <= tex::DEEPEST);
        for style in [0, 1] {
            let laid = tex::Lay::new(&mut ts, 20.0).list(&nodes, style);
            assert!(laid.width.is_finite() && laid.height.is_finite() && laid.depth.is_finite());
            laid.each(0.0, 0.0, &mut |x, y, _| assert!(x.is_finite() && y.is_finite()));
        }
    };
    OPENERS.iter().for_each(|opener| check(opener.repeat(100).as_bytes()));
    run("tex", 1500, Duration::from_secs(2), make, check);
}
