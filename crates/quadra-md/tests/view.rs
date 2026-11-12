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
