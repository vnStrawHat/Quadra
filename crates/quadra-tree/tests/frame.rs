//! What a frame visits for a change: the structural budgets of specs/14 that this stage of
//! the tree can already meet.

mod common;

use common::{Window, block_style};
use quadra_layout::{Display, Length, Overflow};
use quadra_prim::{Color, Rect, Size};
use quadra_reactive::{Scope, signal};
use quadra_tree::{FrameStats, PaintStyle};

fn red() -> PaintStyle {
    PaintStyle { background: Color::rgb(0xFF0000), ..PaintStyle::default() }
}

/// A page: a header block, a card holding a label, and a footer block.
fn page(w: &mut Window) -> (quadra_tree::NodeId, quadra_tree::NodeId, quadra_tree::NodeId) {
    let header = w.block(800.0, 40.0, 0x202020);
    let label = w.text("hello");
    let card = w.block(200.0, 60.0, 0x404040);
    w.tree.set_children(card, &[label]);
    let footer = w.block(800.0, 20.0, 0x606060);
    let root = w.column(&[header, card, footer]);
    w.tree.set_root(root);
    (card, label, footer)
}

#[test]
fn the_first_frame_paints_everything_and_the_next_has_nothing_to_do() {
    let mut w = Window::new();
    page(&mut w);
    assert!(w.tree.needs_frame());
    let (stats, drawn) = w.frame();
    assert_eq!((stats.nodes_painted, stats.layers_rebuilt), (5, 1));
    assert_eq!(drawn.instances_uploaded, 3 + 5); // three blocks, five glyphs
    let (stats, drawn) = w.frame();
    assert_eq!(stats, FrameStats::default());
    assert_eq!((drawn.instances_uploaded, drawn.draw_calls), (0, 1));
}

#[test]
fn a_colour_change_paints_one_node_in_place() {
    let mut w = Window::new();
    let (card, ..) = page(&mut w);
    w.frame();
    w.tree.set_paint_style(card, red());
    let (stats, drawn) = w.frame();
    assert_eq!(
        (stats.nodes_painted, stats.nodes_laid_out, stats.layers_patched, stats.layers_rebuilt),
        (1, 0, 1, 0)
    );
    assert_eq!(drawn.instances_uploaded, 1);
    w.tree.set_paint_style(card, red()); // equal: no frame at all
    assert!(!w.tree.needs_frame());
}

#[test]
fn a_binding_writes_one_property_of_one_node() {
    let mut w = Window::new();
    let (_, label, _) = page(&mut w);
    let name = signal("hello".to_string());
    let handle = w.tree.handle();
    let writer = handle.clone();
    handle.bind(move || writer.set_text(label, name.get()));
    w.frame();

    name.set("world".to_string()); // the same length: the same instances, other glyphs
    assert!(w.tree.needs_frame());
    let (stats, drawn) = w.frame();
    assert_eq!((stats.bindings_run, stats.writes, stats.nodes_painted), (1, 1, 1));
    assert_eq!((stats.layers_patched, drawn.instances_uploaded), (1, 5));
    assert!(stats.nodes_laid_out <= 3, "{stats:?}"); // the label; its card answers as before

    name.set("world".to_string()); // equal: the signal tells nobody
    assert!(!w.tree.needs_frame());
    name.set("worlds".to_string()); // one glyph more: the list is rebuilt
    let (stats, _) = w.frame();
    assert_eq!((stats.nodes_painted, stats.layers_rebuilt), (1, 1));
}

#[test]
fn a_node_that_grows_moves_what_follows_it() {
    let mut w = Window::new();
    let (card, _, footer) = page(&mut w);
    w.frame();
    assert_eq!(w.tree.window_rect(footer), Rect::new(0.0, 100.0, 800.0, 20.0));
    w.tree.set_layout_style(card, block_style(200.0, 90.0, 0).layout);
    let (stats, drawn) = w.frame();
    assert_eq!(w.tree.window_rect(footer).y, 130.0);
    // The card with its label, and the footer that moved; counts are unchanged: in place.
    assert_eq!((stats.nodes_painted, stats.layers_patched, stats.layers_rebuilt), (3, 1, 0));
    assert_eq!(drawn.instances_uploaded, 7);
}

#[test]
fn children_can_be_added_moved_hidden_and_removed() {
    let mut w = Window::new();
    let (card, label, footer) = page(&mut w);
    let root = w.tree.root().unwrap();
    w.frame();
    let extra = w.block(50.0, 10.0, 0x00FF00);
    let mut kids = w.tree.children(root).to_vec();
    kids.insert(1, extra);
    w.tree.set_children(root, &kids);
    let (stats, _) = w.frame();
    assert_eq!(stats.layers_rebuilt, 1);
    assert_eq!(w.tree.window_rect(footer).y, 110.0);

    kids.swap(0, 3); // the footer first, the header last
    w.tree.set_children(root, &kids);
    w.frame();
    assert_eq!(w.tree.window_rect(footer).y, 0.0);

    w.tree.set_children(extra, &[label]); // the label moves from the card into the new block
    w.frame();
    assert_eq!((w.tree.parent(label), w.tree.children(card).len()), (Some(extra), 0));

    let hidden = quadra_layout::Style { display: Display::None, ..w.tree.layout_style(extra).clone() };
    w.tree.set_layout_style(extra, hidden.clone());
    w.frame();
    w.tree.set_layout_style(extra, quadra_layout::Style { display: Display::Flex, ..hidden });
    w.frame();

    w.tree.remove(extra);
    w.frame();
    assert!(!w.tree.is_alive(extra) && !w.tree.is_alive(label));
    assert_eq!(w.tree.len(), 4);
}

#[test]
fn removing_a_node_ends_its_scope() {
    let mut w = Window::new();
    let (_, label, _) = page(&mut w);
    let name = signal("a".to_string());
    let scope = Scope::child();
    let handle = w.tree.handle();
    scope.run(|| {
        let writer = handle.clone();
        handle.bind(move || writer.set_text(label, name.get()));
    });
    w.tree.set_scope(label, scope);
    w.frame();
    w.tree.remove(label);
    w.frame();
    name.set("b".to_string()); // its binding is gone: nothing is due
    assert!(!scope.is_alive() && !w.tree.needs_frame());
}

#[test]
fn a_new_scale_paints_everything_again_and_a_new_size_only_lays_out() {
    let mut w = Window::new();
    let (card, ..) = page(&mut w);
    w.frame();
    w.scale = 1.5;
    let (stats, _) = w.frame();
    assert_eq!((stats.nodes_painted, stats.nodes_laid_out), (5, 0));
    let scene = w.tree.scene();
    let card_quad = scene.layer(scene.roots()[0]).instances()[1];
    assert_eq!(card_quad.rect(), Rect::new(0.0, 60.0, 300.0, 90.0)); // device pixels
    w.size = Size::new(640.0, 480.0);
    let (stats, _) = w.frame();
    // Only the root changed size, and it draws nothing: no instance is written.
    assert!(stats.nodes_laid_out > 0 && stats.instances_written == 0, "{stats:?}");
    assert_eq!(w.tree.rect(card).width, 200.0);
}

#[test]
fn one_change_among_ten_thousand_nodes_costs_one_node() {
    let mut w = Window::new();
    let rows: Vec<_> = (0..10_000).map(|i| w.block(100.0, 2.0, 0x101010 + i as u32)).collect();
    let root = w.column(&rows);
    // A scrolling list: its rows keep their height instead of shrinking into the window.
    let scrolls = quadra_layout::Style { overflow_y: Overflow::Scroll, ..w.tree.layout_style(root).clone() };
    w.tree.set_layout_style(root, scrolls);
    w.tree.set_root(root);
    let (stats, drawn) = w.frame();
    assert_eq!((stats.nodes_painted, drawn.instances_uploaded), (10_001, 10_000)); // the list itself draws nothing
    w.tree.set_paint_style(rows[7_000], red());
    let (stats, drawn) = w.frame();
    assert_eq!((stats.nodes_painted, stats.nodes_laid_out, drawn.instances_uploaded), (1, 0, 1));
    // A fixed-size row that changes its content size: the list above it never runs.
    let wider = quadra_layout::Style { width: Length::Px(120.0), ..w.tree.layout_style(rows[5]).clone() };
    w.tree.set_layout_style(rows[5], wider);
    let (stats, drawn) = w.frame();
    assert_eq!((stats.nodes_painted, drawn.instances_uploaded), (1, 1));
    assert!(stats.nodes_laid_out < 10, "{stats:?}");
}
