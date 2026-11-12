//! The parser of formulas: the nodes of what is written, the faults it tells, and every
//! name of its tables.

use super::alphabets::{ACCENTS, FONTS, SPACES};
use super::commands::ENVIRONMENTS;
use super::symbols::{BIG_OPS, FUNCTIONS, SYMBOLS};
use super::{Class, Limits, List, Node, parse};

fn nodes(source: &str) -> List {
    let (nodes, fault) = parse(source);
    assert_eq!(fault, None, "{source}");
    nodes
}

fn atom(text: &str, class: Class) -> Node {
    Node::Atom { text: text.into(), class }
}

fn ord(text: &str) -> Node {
    atom(text, Class::Ord)
}

#[test]
fn a_formula_is_its_nodes() {
    // Letters are slanted and digits upright; a minus is a minus.
    assert_eq!(nodes("x - 1"), [ord("𝑥"), atom("−", Class::Bin), ord("1")]);
    let scripts =
        Node::Scripts {
            base: Box::new(ord("𝑥")), sup: Some(vec![ord("2")]), sub: Some(vec![ord("𝑖")])
        };
    assert_eq!(nodes("x^2_i"), std::slice::from_ref(&scripts));
    assert_eq!(nodes("x_i^{2}"), [scripts]);
    // Primes are one superscript, and what is raised after them joins it.
    let primed =
        Node::Scripts { base: Box::new(ord("𝑓")), sup: Some(vec![ord("″"), ord("2")]), sub: None };
    assert_eq!(nodes("f''^2"), [primed]);
    let frac = Node::Frac { over: vec![ord("1")], under: vec![ord("2")], rule: true, style: Some(0) };
    assert_eq!(nodes(r"\dfrac12"), [frac]);
    assert_eq!(nodes(r"\sqrt[3]{x}"), [Node::Sqrt { index: Some(vec![ord("3")]), body: vec![ord("𝑥")] }]);
    assert_eq!(nodes(r"\left( x \right."), [Node::Delim { left: "(", right: "", body: vec![ord("𝑥")] }]);
    assert_eq!(
        nodes(r"\alpha \Gamma \le \mathrm{d}"),
        [ord("𝛼"), ord("Γ"), atom("≤", Class::Rel), Node::Group(vec![ord("d")])]
    );
    assert_eq!(
        nodes(r"\mathbb{R1} \mathcal{L} \mathbf{x}"),
        [Node::Group(vec![ord("ℝ"), ord("𝟙")]), Node::Group(vec![ord("ℒ")]), Node::Group(vec![ord("𝐱")]),]
    );
    assert_eq!(
        nodes(r"\sum\nolimits \lim \int\limits"),
        [
            Node::BigOp { glyph: "∑", limits: Limits::Side },
            Node::Function { name: "lim".into(), limits: Limits::Display },
            Node::BigOp { glyph: "∫", limits: Limits::Always },
        ]
    );
    assert_eq!(
        nodes(r"\operatorname*{arg\,max} \text{if } x % no more"),
        [
            Node::Function { name: "arg max".into(), limits: Limits::Display },
            Node::Text("if ".into()),
            ord("𝑥"),
        ]
    );
    assert_eq!(
        nodes(r"\not= \not\in \bar x \overline{x}"),
        [
            atom("≠", Class::Rel),
            atom("∉", Class::Rel),
            Node::Accent { mark: "¯", body: vec![ord("𝑥")] },
            Node::Line { body: vec![ord("𝑥")], under: false },
        ]
    );
}

#[test]
fn an_environment_is_rows_of_cells() {
    let cell = |text: &str| vec![ord(text)];
    let rows = vec![vec![cell("𝑎"), cell("𝑏")], vec![cell("𝑐"), cell("𝑑")]];
    let matrix = Node::Matrix { env: "pmatrix", rows };
    assert_eq!(nodes(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}"), std::slice::from_ref(&matrix));
    // A `\\` after the last row is no row, and one may say how far the next is.
    assert_eq!(nodes(r"\begin{pmatrix} a & b \\[4pt] c & d \\ \end{pmatrix}"), [matrix]);
    assert!(
        matches!(&nodes(r"\begin{array}{cc} 1 & 2 \end{array}")[..], [Node::Matrix { env: "array", rows }] if rows.len() == 1)
    );
    // A cell outside rows ends the formula.
    assert_eq!(parse("a & b"), (vec![ord("𝑎")], Some("unexpected &".into())));
}

#[test]
fn the_first_fault_is_told_and_the_rest_is_read() {
    let faults = [
        (r"\frac{a}", "missing argument"),
        (r"\foo x", r"unknown command \foo"),
        ("x^2^3", "double superscript"),
        ("x_2_3", "double subscript"),
        ("{x", "missing }"),
        ("x}", "unexpected }"),
        (r"\sqrt[3 x", "missing ]"),
        (r"\left( x", r"\left without \right"),
        (r"\left< x \right>", "bad delimiter <"),
        (r"\begin{foo} x \end{foo}", "unknown environment foo"),
        (r"\begin{matrix} x \end{cases}", r"\end does not match \begin{matrix}"),
        (r"\begin{matrix} x", r"missing \end{matrix}"),
        (r"a \\ b", r"unexpected \"),
    ];
    for (source, fault) in faults {
        assert_eq!(parse(source).1.as_deref(), Some(fault), "{source}");
    }
    // What is around a fault is there all the same.
    let (nodes, _) = parse(r"a + \foo + b");
    assert_eq!(nodes.len(), 5);
    assert_eq!(nodes[2], Node::Text(r"\foo".into()));
}

#[test]
fn every_name_of_the_tables_is_read() {
    let mut sources = Vec::new();
    sources.extend(SYMBOLS.iter().map(|row| format!("\\{} ", row.0)));
    sources.extend(BIG_OPS.iter().map(|row| format!("\\{}_a^b x", row.0)));
    sources.extend(FUNCTIONS.iter().map(|row| format!("\\{} x", row.0)));
    sources.extend(ACCENTS.iter().map(|row| format!("\\{}{{x}}", row.0)));
    sources.extend(FONTS.iter().map(|row| format!("\\{}{{Ab1}}", row.0)));
    sources.extend(SPACES.iter().map(|row| format!("a\\{} b", row.0)));
    sources.extend(ENVIRONMENTS.iter().map(|env| {
        let columns = if *env == "array" { "{cc}" } else { "" };
        format!("\\begin{{{env}}}{columns} a & b \\\\ c & d \\end{{{env}}}")
    }));
    let more = [
        r"\frac{a}{b}",
        r"\tfrac{1}{2}",
        r"\binom{n}{k}",
        r"\left\{ x \right\}",
        r"\left\langle x \right.",
        r"\bigl( x \bigr)",
        r"\Bigg[ x \Bigg]",
        r"\biggm| x",
        r"\overset{!}{=}",
        r"\underset{x}{\arg\max}",
        r"\pmod{n}",
        "f''(x)",
        r"\displaystyle x \scriptstyle y",
        r"\lim\limits_{x} \int\nolimits_0^1",
        r"\{ \} \| \% \$ \# \& \_",
        r"\textcolor{red}{x} \color{red} y",
        "^2 _i '",
    ];
    sources.extend(more.map(str::to_string));
    for source in sources {
        assert_eq!(parse(&source).1, None, "{source}");
    }
}

#[test]
fn a_name_is_in_its_table_once() {
    let mut names: Vec<&str> = SYMBOLS.iter().map(|row| row.0).collect();
    names.extend(BIG_OPS.iter().map(|row| row.0));
    names.extend(FUNCTIONS.iter().map(|row| row.0));
    names.extend(ACCENTS.iter().map(|row| row.0));
    names.extend(FONTS.iter().map(|row| row.0));
    names.extend(SPACES.iter().map(|row| row.0));
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), count);
}
