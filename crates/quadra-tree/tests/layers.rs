//! Layers: scrolling and moving change one constant and paint nothing (specs/07, "Layers";
//! the scroll rows of specs/14's structural budgets).

mod common;

use common::Window;
use quadra_layout::{Direction, Length, Overflow};
use quadra_prim::{Affine, Color, Point, Rect};
use quadra_render::{Renderer, Step};
use quadra_tree::{NodeId, PaintStyle};

struct Page {
    w: Window,
    header: NodeId,
    viewport: NodeId,
    content: NodeId,
    rows: Vec<NodeId>,
}

/// A header, then a 200 x 100 viewport scrolling a list of 50 rows of 20 px. The list is a
/// layer cut to the viewport.
fn page() -> Page {
    let mut w = Window::new();
    let header = w.block(800.0, 40.0, 0x202020);
    let rows: Vec<_> = (0..50).map(|i| w.block(180.0, 20.0, 0x300000 + i)).collect();
    let content = w.column(&rows);
    w.tree.make_layer(content, true);
    let viewport = w.block(200.0, 100.0, 0x101010);
    let scrolls = quadra_layout::Style {
        direction: Direction::Column,
        overflow_y: Overflow::Scroll,
        ..w.tree.layout_style(viewport).clone()
    };
    w.tree.set_layout_style(viewport, scrolls);
    w.tree.set_children(viewport, &[content]);
    let root = w.column(&[header, viewport]);
    w.tree.set_root(root);
    w.frame();
    Page { w, header, viewport, content, rows }
}

/// The transform and scissor the renderer was given for the layer drawn second.
fn second_layer(w: &Window) -> (Affine, Rect, f32) {
    let layers: Vec<_> = w
        .renderer
        .steps()
        .iter()
        .filter_map(|step| match step {
            Step::Layer { world, .. } => Some((world.transform, world.scissor, world.opacity)),
            Step::Run { .. } => None,
        })
        .collect();
    layers[1]
}

#[test]
fn scrolling_paints_nothing_and_uploads_nothing() {
    let mut p = page();
    assert_eq!(p.w.tree.rect(p.content).height, 1000.0); // the rows keep their height
    assert_eq!(second_layer(&p.w), (Affine::translate(0.0, 40.0), Rect::new(0.0, 40.0, 200.0, 100.0), 1.0));
    assert_eq!(p.w.renderer.stats().draw_calls, 2); // the page, and the list inside it

    p.w.tree.set_layer_offset(p.content, Point::new(0.0, -260.0));
    assert!(p.w.tree.needs_frame());
    let (stats, drawn) = p.w.frame();
    assert_eq!((stats.nodes_painted, stats.nodes_laid_out, stats.layers_composed), (0, 0, 1));
    assert_eq!((drawn.instances_uploaded, drawn.layers_uploaded), (0, 0));
    assert_eq!(second_layer(&p.w).0, Affine::translate(0.0, -220.0));

    p.w.tree.set_layer_offset(p.content, Point::new(0.0, -260.0)); // equal: no frame
    assert!(!p.w.tree.needs_frame());
    p.w.tree.set_layer_opacity(p.content, 0.5); // a fade is a constant too
    let (stats, drawn) = p.w.frame();
    assert_eq!((stats.nodes_painted, stats.layers_composed, drawn.instances_uploaded), (0, 1, 0));
    assert_eq!(second_layer(&p.w).2, 0.5);
}

#[test]
fn a_layer_that_moves_keeps_its_instances() {
    let mut p = page();
    let taller = quadra_layout::Style { height: Length::Px(70.0), ..p.w.tree.layout_style(p.header).clone() };
    p.w.tree.set_layout_style(p.header, taller);
    let (stats, drawn) = p.w.frame();
    // The header and the viewport box are painted again; the fifty rows are not.
    assert_eq!((stats.nodes_painted, stats.layers_composed), (2, 1));
    assert_eq!(drawn.instances_uploaded, 3); // two boxes, and the marker of the list, written again
    assert_eq!(second_layer(&p.w).1, Rect::new(0.0, 70.0, 200.0, 100.0)); // cut to the moved box
    assert_eq!(p.w.tree.window_rect(p.viewport).y, 70.0);
}

#[test]
fn a_change_inside_a_layer_touches_only_that_layer() {
    let mut p = page();
    let red = PaintStyle { background: Color::rgb(0xFF0000), ..PaintStyle::default() };
    p.w.tree.set_paint_style(p.rows[30], red);
    let (stats, drawn) = p.w.frame();
    assert_eq!((stats.nodes_painted, stats.layers_patched, stats.layers_rebuilt), (1, 1, 0));
    assert_eq!((drawn.instances_uploaded, drawn.layers_uploaded), (1, 1));

    p.w.tree.remove(p.rows[10]); // the list is rebuilt; the page around it is not touched
    let (stats, drawn) = p.w.frame();
    assert_eq!((stats.layers_rebuilt, drawn.layers_uploaded), (1, 1));
    assert_eq!(drawn.instances_uploaded, 49);
}

#[test]
fn a_node_can_become_a_layer_and_stop_being_one() {
    let mut p = page();
    assert_eq!(p.w.tree.scene().len(), 2);
    p.w.tree.make_layer(p.header, false);
    p.w.tree.make_layer(p.rows[3], false); // a layer inside a layer
    p.w.frame();
    assert_eq!((p.w.tree.scene().len(), p.w.renderer.stats().layers_bound), (4, 5));
    p.w.tree.set_layer_offset(p.rows[3], Point::new(5.5, 0.0)); // whole device pixels: 6
    let (stats, _) = p.w.frame();
    assert_eq!((stats.nodes_painted, stats.layers_composed), (0, 1));

    p.w.tree.remove_layer(p.header);
    p.w.tree.remove_layer(p.content);
    p.w.frame();
    assert!(!p.w.tree.is_layer(p.content) && p.w.tree.is_layer(p.rows[3]));
    assert_eq!(p.w.tree.scene().len(), 2);

    p.w.tree.remove(p.viewport); // with the layer inside it
    p.w.frame();
    assert_eq!((p.w.tree.scene().len(), p.w.renderer.buffers()), (1, 1));
}
