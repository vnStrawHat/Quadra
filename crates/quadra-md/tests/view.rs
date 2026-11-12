//! The view of a document (specs/13, "Markdown"): what a reader is told, links that wait for
//! Ctrl, columns that line up, and blocks that keep their nodes while their source does.

use std::rc::Rc;
use std::time::Duration;

use quadra_kit::dark;
use quadra_md::markdown;
use quadra_platform::{Mods, MouseButton};
use quadra_prim::Point;
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_testing::fuzz::{Rng, mutate, run};
use quadra_tree::{PointerInput, Role};

const NOTES: &str = "# Release *0.4*\n\nSee the [notes](https://example.com/up) and ~~wait~~.\n\n- [x] back up\n- [ ] upgrade\n\n> gone: `--legacy`\n\n| Setting | Default |\n|---|--:|\n| `timeout` | 30 s |\n| retries | 3 |\n\n```sh\nmake all\n```\n";

fn page(source: &str) -> (Harness, Signal<String>, Signal<String>) {
    let (source, clicked) = (signal(source.to_string()), signal(String::new()));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let doc = markdown(source).on_link(move |target| clicked.set(target.to_string()));
        quadra_kit::root().child(doc)
    });
    h.frame();
    (h, source, clicked)
}

/// The names of the nodes of a role, in the order of the tree.
fn named(h: &Harness, role: Role) -> Vec<String> {
    let tree = h.tree.semantic_tree();
    tree.nodes.iter().filter(|n| n.semantics.role == role).map(|n| n.semantics.name.clone()).collect()
}

#[test]
fn a_document_is_what_a_reader_is_told_and_its_links_wait_for_ctrl() {
    let (mut h, _, clicked) = page(NOTES);
    assert_eq!(named(&h, Role::Heading), ["Release 0.4"]);
    assert!(h.has_text("See the notes and wait.") && h.has_text("make all") && h.has_text("gone: --legacy"));
    // Tasks are boxes that say whether they are done.
    let tree = h.tree.semantic_tree();
    let boxes = tree.nodes.iter().filter(|n| n.semantics.role == Role::Checkbox);
    let boxes: Vec<_> = boxes.map(|n| (n.semantics.name.as_str(), n.semantics.checked)).collect();
    assert_eq!(boxes, [("back up", Some(true)), ("upgrade", Some(false))]);

    // The cells of a column start at the same place, and the table is as wide as the page.
    let cells = h.query(|n| n.semantics.role == Role::Cell);
    let at = |k: usize| h.tree.window_rect(cells[k]);
    assert_eq!(cells.len(), 6);
    assert!(
        (0..3)
            .all(|r| at(2 * r).x == at(0).x && at(2 * r + 1).x == at(1).x && at(2 * r).y == at(2 * r + 1).y)
    );
    assert!(at(0).x == 1.0 && at(1).x + at(1).width == 799.0, "{:?} {:?}", at(0), at(1));

    // "notes" is the third word of the paragraph under the heading: a click on it opens
    // nothing, a click with Ctrl held gives where it leads.
    let words = h.get_by_text("See the notes and wait.").node();
    let link = h.tree.window_rect(words);
    let (x, y) = (link.x + 8.0 * 9.6 + 12.0, link.y + 12.0);
    h.click(x, y);
    assert_eq!(clicked.get_untracked(), "");
    for input in [PointerInput::Move, PointerInput::Down, PointerInput::Up] {
        h.tree.pointer(input, MouseButton::Left, Point::new(x, y), Mods::CTRL, h.now());
    }
    assert_eq!(clicked.get_untracked(), "https://example.com/up");
}

#[test]
fn a_formula_is_drawn_in_the_middle_and_one_with_a_fault_shows_its_source() {
    let (mut h, source, _) = page("Euler:\n\n$$\ne^{i\\pi} + 1 = 0\n$$\n\n$$\n\\frac{a}\n$$\n");
    // To a reader a formula is its source; on the screen it is glyphs, in the middle.
    let formula = h.get_by_text("e^{i\\pi} + 1 = 0").node();
    let (at, glyphs) = (h.tree.window_rect(formula), painted(&h));
    assert!(at.width > 60.0 && at.height > 14.0, "{at:?}");
    assert!((at.x + at.width / 2.0 - 400.0).abs() < 1.0, "{at:?}");
    assert!(glyphs > "Euler:".len() + 8, "{glyphs}");
    // What is wrong is said under the source, and the rest of the document is there.
    assert!(h.has_text("\\frac{a}") && h.has_text("Math: missing argument") && h.has_text("Euler:"));
    // A formula in a line is in the text, said by its source; with a fault it is its source.
    source.set("The sum $a^2+b^2$ and $\\frac{a}$ here.".to_string());
    h.frame();
    assert!(h.has_text("The sum a^2+b^2 and \\frac{a} here."));
    // A fraction is a rule and two lists: higher than a line of text.
    source.set("$$\n\\frac{a}{b}\n$$\n".to_string());
    h.frame();
    let fraction = h.get_by_text("\\frac{a}{b}").node();
    assert!(h.tree.window_rect(fraction).height > 30.0);
}

/// How many instances the layers of the window hold.
fn painted(h: &Harness) -> usize {
    let layers = h.renderer.steps().iter().filter_map(|step| match step {
        quadra_render::Step::Layer { id, .. } => Some(h.tree.scene().layer(*id).instances().len()),
        _ => None,
    });
    layers.sum()
}

#[test]
fn a_block_keeps_its_nodes_while_its_source_does() {
    let (mut h, source, _) = page(NOTES);
    let node = |h: &mut Harness, role: Role| h.query(|n| n.semantics.role == role)[0];
    let (heading, table, count) = (node(&mut h, Role::Heading), node(&mut h, Role::Table), h.tree.len());
    // A word more in the paragraph: its block is built again, the others stay as they are,
    // though every block after it is a line further down in the source.
    source.set(NOTES.replace("and ~~wait~~.", "and ~~wait~~\nfor it."));
    let (stats, _) = h.frame();
    assert!(h.has_text("See the notes and wait for it."));
    assert_eq!(
        (node(&mut h, Role::Heading), node(&mut h, Role::Table), h.tree.len()),
        (heading, table, count)
    );
    assert!(stats.nodes_painted < 12, "{stats:?}");
    // A block more at the start: nothing that was there is built again.
    source.update(|source| source.insert_str(0, "intro\n\n"));
    h.frame();
    assert_eq!((node(&mut h, Role::Heading), node(&mut h, Role::Table)), (heading, table));
    // Nothing: nothing.
    source.set(String::new());
    h.frame();
    assert!(named(&h, Role::Heading).is_empty() && h.tree.len() < 12);
}

#[test]
fn nothing_a_document_can_hold_makes_the_view_fail() {
    let corpus = [
        NOTES,
        "#\n\n-\n\n> \n\n``\n\n|\n|-|\n\n$$\n$$\n\n---\ntitle\n---\n",
        ">>> deep\n> > spaced\n\t- tab item\n    code?\n***\n___\n| x |\n|:--|\n",
        "[link](<http://a b>) [ref][1] http://bare.example/x?y=1. *a _b* c_ **unclosed ![i](a.png)",
        "Xin chào **thế giới** 漢字 😀 é\r\n\r\n1. a\r\n2. b\r\n```\nsee https://x.io/a\ttab\n```",
    ]
    .map(str::as_bytes);
    let make = |rng: &mut Rng, round: usize| match round % 8 {
        0 => format!("{} q\n{}- item", ">".repeat(2000), " ".repeat(2000)).into_bytes(),
        1 => format!("|{}\n|{}\n", "h|".repeat(300), "-|".repeat(300)).into_bytes(),
        _ => {
            let from = *rng.pick(&corpus);
            mutate(rng, from, &corpus)
        }
    };
    run("the view of markdown", 200, Duration::from_secs(5), make, |input| {
        let (mut h, source, _) = page(&String::from_utf8_lossy(input));
        source.update(|source| source.push_str("\n\nmore"));
        h.frame();
    });
}

#[test]
fn a_diagram_is_drawn_in_the_middle_and_one_that_is_not_shows_its_source() {
    let (mut h, source, _) = page("A flow:\n\n```mermaid\ngraph TD\nA[Start] --> B{Choice}\n```\n");
    // To a reader a diagram is a picture named by its kind and what its nodes say.
    let diagram = h.get_by_role(Role::Image, "Flowchart: Start, Choice").node();
    let at = h.tree.window_rect(diagram);
    assert!(at.width > 80.0 && at.height > 150.0, "{at:?}");
    assert!((at.x + at.width / 2.0 - 400.0).abs() < 1.0, "{at:?}");
    // On the screen its link and the outline of the diamond are strokes: a segment an instance.
    let strokes = |h: &Harness| {
        let layers = h.renderer.steps().iter().filter_map(|step| match step {
            quadra_render::Step::Layer { id, .. } => Some(h.tree.scene().layer(*id).instances()),
            _ => None,
        });
        layers.flatten().filter(|instance| instance.kind == quadra_render::Kind::Line as u32).count()
    };
    assert!(strokes(&h) >= 5, "{}", strokes(&h));
    // A kind that is not drawn, and a source with a fault, are their source and why.
    source.set("```mermaid\npie title Pets\n```\n".to_string());
    h.frame();
    assert!(h.has_text("pie title Pets") && h.has_text("Diagram: diagram kind `pie` is not supported"));
    assert_eq!(strokes(&h), 0);
    source.set("```mermaid\ngraph TD\nA -->\n```\n".to_string());
    h.frame();
    assert!(h.has_text("Diagram: line 2: expected a node at ``"));
    // A sequence diagram is named by its participants; its lifelines are dashes.
    source.set("```mermaid\nsequenceDiagram\nactor U as User\nU->>App: Save\n```\n".to_string());
    h.frame();
    let sequence = h.get_by_role(Role::Image, "Sequence diagram: User, App").node();
    assert!(h.tree.window_rect(sequence).height > 150.0 && strokes(&h) > 20, "{}", strokes(&h));
}

#[test]
fn a_paragraph_of_images_shows_them_and_one_that_cannot_show_is_its_words() {
    // A file beside the document; the harness decodes flat test images.
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/md-images");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("chart.flat"), quadra_platform::FlatImages::file(1600, 400, 0x2040_60ff))
        .unwrap();
    let source = "Before.\n\n![A chart](chart.flat)\n\n![Gone](gone.png) ![Far](https://example.com/far.png)\n\nWords and ![a mark](chart.flat) in a line.\n";
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || quadra_kit::root().child(markdown(source).base(folder)));
    h.settle();

    // Wider than the page: as wide as the page, in its own shape.
    let chart = h.get_by_role(Role::Image, "A chart").node();
    let chart = h.tree.rect(chart);
    assert!(chart.width > 400.0 && chart.width <= 800.0 && chart.height == chart.width / 4.0, "{chart:?}");
    // No such file, and a file elsewhere, which is never fetched: their words and where.
    assert!(h.has_text("Gone (gone.png)") && h.has_text("Far (https://example.com/far.png)"));
    assert_eq!(named(&h, Role::Image), ["A chart", "Gone"], "the one that failed is an empty box");
    // Among words an image is its words.
    assert!(h.has_text("Words and [a mark] in a line."));
}
