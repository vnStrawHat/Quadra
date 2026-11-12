//! The parser of Markdown (specs/13, "Markdown"; behaviour/21): every kind of block with its
//! source line, and every kind of inline.

use quadra_md::{Align, Block, Kind, Span, anchor, block_at, outline, parse, parse_inline, plain, words};

/// A word for the kind of a block, and its text.
fn said(block: &Block) -> (&'static str, &str) {
    match &block.kind {
        Kind::Paragraph { text } => ("paragraph", text),
        Kind::Heading { text, .. } => ("heading", text),
        Kind::Item { number: None, text, .. } => ("bullet", text),
        Kind::Item { text, .. } => ("ordered", text),
        Kind::Code { text, .. } => ("code", text),
        Kind::Quote { text, .. } => ("quote", text),
        Kind::Rule => ("rule", ""),
        Kind::Table { .. } => ("table", ""),
        Kind::Math { text } => ("math", text),
        Kind::Front { text } => ("front", text),
    }
}

fn kinds(blocks: &[Block]) -> Vec<&'static str> {
    blocks.iter().map(|block| said(block).0).collect()
}

fn texts(spans: &[Span]) -> Vec<&str> {
    spans.iter().map(|span| span.text.as_str()).collect()
}

#[test]
fn a_document_is_its_blocks() {
    let source = "# Title\n\nSome *text*\ncontinued.\n\n- one\n- two\n  - nested\n1. first\n\n```v\nfn main() {}\n```\n> quoted\n> more\n\n---\n| A | B |\n|---|:-:|\n| 1 | 2 |\n";
    let b = parse(source);
    let expected =
        ["heading", "paragraph", "bullet", "bullet", "bullet", "ordered", "code", "quote", "rule", "table"];
    assert_eq!(kinds(&b), expected);
    assert_eq!(b[0].kind, Kind::Heading { level: 1, text: "Title".into() });
    assert_eq!(said(&b[1]).1, "Some *text* continued.");
    assert_eq!(b[4].kind, Kind::Item { number: None, depth: 1, task: None, text: "nested".into() });
    assert!(matches!(b[5].kind, Kind::Item { number: Some(1), depth: 0, .. }));
    assert_eq!(b[6].kind, Kind::Code { lang: "v".into(), text: "fn main() {}".into() });
    assert_eq!(b[7].kind, Kind::Quote { depth: 1, text: "quoted more".into() });
    let cell = |cells: [&str; 2]| cells.map(String::from).to_vec();
    let table = Kind::Table {
        align: vec![Align::None, Align::Center],
        rows: vec![cell(["A", "B"]), cell(["1", "2"])],
    };
    assert_eq!(b[9].kind, table);
    // Each knows the line it starts on.
    assert_eq!(b.iter().map(|block| block.line).collect::<Vec<_>>(), [0, 2, 5, 6, 7, 8, 10, 13, 16, 17]);
}

#[test]
fn tasks_math_the_front_matter_and_quotes_in_quotes() {
    let source = "---\ntitle: x\n---\n# A\n\n- [ ] open\n- [x] done\n\n$$\n\\frac{a}{b}\n$$\n\n$$ x^2 $$\n```math\ny\n```\n> > deep\n> top\n";
    let b = parse(source);
    assert_eq!(kinds(&b), ["front", "heading", "bullet", "bullet", "math", "math", "math", "quote", "quote"]);
    assert_eq!((said(&b[0]).1, b[1].line), ("title: x", 3));
    assert_eq!(b[2].kind, Kind::Item { number: None, depth: 0, task: Some(false), text: "open".into() });
    assert_eq!(b[2].line, 5);
    assert_eq!(b[3].kind, Kind::Item { number: None, depth: 0, task: Some(true), text: "done".into() });
    assert_eq!((said(&b[4]).1, b[4].line), ("\\frac{a}{b}", 8));
    assert_eq!((said(&b[5]).1, b[5].line), ("x^2", 12));
    assert_eq!(said(&b[6]).1, "y");
    assert_eq!(b[7].kind, Kind::Quote { depth: 2, text: "deep".into() });
    assert!(matches!(b[8].kind, Kind::Quote { depth: 1, .. }));
    // A front matter that never ends is not one; a fence that never ends runs to the end.
    assert_eq!(kinds(&parse("---\ntitle: x\n")), ["rule", "paragraph"]);
    assert_eq!(
        parse("~~~Rust {.numbered}\nlet a;\n```\nstill")[0].kind,
        Kind::Code { lang: "rust".into(), text: "let a;\n```\nstill".into() }
    );
}

#[test]
fn tables_align_lines_break_and_tabs_nest() {
    let b = parse("| a | b | c |\n|:--|:-:|--:|\n| 1 | 2 | 3 |\n\nline one  \nline two\\\nthree\nfour");
    assert!(
        matches!(&b[0].kind, Kind::Table { align, .. } if align[..] == [Align::Left, Align::Center, Align::Right])
    );
    assert_eq!((said(&b[1]).1, b[1].line), ("line one\nline two\nthree four", 4));
    let depths = |source: &str| -> Vec<usize> {
        let depth = |block: &Block| match block.kind {
            Kind::Item { depth, .. } => depth,
            _ => usize::MAX,
        };
        parse(source).iter().map(depth).collect()
    };
    assert_eq!(depths("- a\n\t- b\n\t\t- c"), [0, 2, 4]);
    // A line that is indented goes on with the item; one that is not is a paragraph.
    assert_eq!(kinds(&parse("- a\n  more\nnot")), ["bullet", "paragraph"]);
    assert_eq!(said(&parse("- a\n  more\nnot")[0]).1, "a more");
    // What looks like a block and is not: a hashtag, a number without a space, two dashes.
    assert_eq!(kinds(&parse("#tag\n\n1.5 is a number\n\n--\n\n####### seven")), ["paragraph"; 4]);
}

#[test]
fn comments_show_nothing() {
    let b =
        parse("a\n\n<!-- one\ntwo -->\n<!-- x --> gone\nb <!-- hidden --> c\n\n```\n<!-- kept -->\n```\n");
    assert_eq!(kinds(&b), ["paragraph", "paragraph", "code"]);
    assert_eq!(said(&b[1]).1, "b <!-- hidden --> c");
    assert_eq!(plain(said(&b[1]).1), "b  c");
    assert_eq!(said(&b[2]).1, "<!-- kept -->");
    assert_eq!(parse_inline("`<!-- x -->`")[0].text, "<!-- x -->");
}

#[test]
fn a_text_is_its_spans() {
    let s = parse_inline("a **b *c*** `d*e` [f **g**](http://x) snake_case_name <https://y>");
    assert_eq!(texts(&s), ["a ", "b ", "c", " ", "d*e", " ", "f ", "g", " snake_case_name ", "https://y"]);
    assert!(s[1].bold && !s[1].italic && s[2].bold && s[2].italic && s[4].code);
    assert_eq!(s[6].link.as_deref(), Some("http://x"));
    assert!(s[7].bold && s[7].link.as_deref() == Some("http://x"));
    assert_eq!((s[8].link.as_deref(), s[9].link.as_deref()), (None, Some("https://y")));
    assert_eq!(texts(&parse_inline("2 * 3 * 4")), ["2 * 3 * 4"]);
    assert_eq!(texts(&parse_inline("\\*not\\*")), ["*not*"]);

    let s = parse_inline("~~old~~ new $x^2$ and $5 or $10 ![logo](img/a.png \"t\") see https://x.io/a, ok");
    assert!(s[0].strike && s[0].text == "old" && !s[1].strike);
    let math: Vec<&Span> = s.iter().filter(|span| span.math).collect();
    assert!(math.len() == 1 && math[0].text == "x^2");
    assert!(s.iter().any(|span| span.text.contains("$5 or $10")));
    let images: Vec<&Span> = s.iter().filter(|span| span.image.is_some()).collect();
    assert!(images.len() == 1 && images[0].image.as_deref() == Some("img/a.png") && images[0].text == "logo");
    let links: Vec<&Span> = s.iter().filter(|span| span.link.is_some()).collect();
    assert!(links.len() == 1 && links[0].link.as_deref() == Some("https://x.io/a"));
    let nested = parse_inline("[a [b] c](u)");
    assert!(nested.len() == 1 && nested[0].text == "a [b] c" && nested[0].link.as_deref() == Some("u"));
}

#[test]
fn links_in_the_labels_of_links_stop_and_what_is_not_closed_is_text() {
    // Twenty deep: past sixteen, the label is as written.
    let deep = format!("{}x{}", "[".repeat(20), "](u)".repeat(20));
    let spans = parse_inline(&deep);
    assert_eq!(spans.len(), 1);
    assert_eq!((spans[0].text.as_str(), spans[0].link.as_deref()), ("[[[[x](u)](u)](u)](u)", Some("u")));
    // Nothing closes any of these.
    for open in ["`a", "[a](b", "[a", "<!-- a", "<https://a", "**a", "$a"] {
        assert_eq!(plain(open).replace("**", ""), open.replace("**", ""), "{open}");
    }
    // Letters that are not ASCII are whole in every span.
    assert_eq!(
        texts(&parse_inline("chào **thế giới** 漢字 _é_ 😀")),
        ["chào ", "thế giới", " 漢字 ", "é", " 😀"]
    );
}

#[test]
fn the_outline_is_the_headings_with_their_anchors() {
    let blocks = parse("# Hello, *World*\n\ntext\n\n## Cài đặt & chạy\n\nmore words here\n");
    let headings = outline(&blocks);
    let of: Vec<_> =
        headings.iter().map(|h| (h.level, h.text.as_str(), h.line, h.block, h.anchor.as_str())).collect();
    assert_eq!(of, [(1, "Hello, World", 0, 0, "hello-world"), (2, "Cài đặt & chạy", 4, 2, "cài-đặt--chạy")]);
    assert_eq!(anchor("  A_b - C!  "), "a_b---c");
    // The block of a line: the one it is in, or the one before a blank line.
    assert_eq!([0, 1, 2, 3, 5, 6, 99].map(|line| block_at(&blocks, line)), [0, 0, 1, 1, 2, 3, 3]);
    assert_eq!(block_at(&[], 3), 0);
    assert_eq!(words("Đọc 3 lần, rồi viết-lại!"), 6);
}
