//! Styles with state variants (specs/05, "Style and states"; specs/13): a state that changes
//! on a node with a variant for it resolves the node's style again, and the difference decides
//! what follows. A state nothing styles costs nothing.

mod common;

use std::rc::Rc;

use common::Window;
use quadra_platform::{Mods, MouseButton};
use quadra_prim::{Color, Point};
use quadra_tree::{Cast, NodeId, Patch, PointerInput, State, Theme, Token};

const GRAY: u32 = 0x808080;
const LIGHT: u32 = 0xA0A0A0;
const DARK: u32 = 0x404040;

struct Page {
    w: Window,
    plain: NodeId,
    button: NodeId,
}

/// Two 100 x 40 boxes in a column: a plain one, and below it a button that is lighter under
/// the pointer and darker while pressed.
fn page() -> Page {
    let mut w = Window::new();
    let (plain, button) = (w.block(100.0, 40.0, 0x202020), w.block(100.0, 40.0, GRAY));
    w.tree.set_variant(button, State::HOVER, vec![Patch::Background(Color::rgb(LIGHT).into())]);
    w.tree.set_variant(button, State::ACTIVE, vec![Patch::Background(Color::rgb(DARK).into())]);
    let root = w.column(&[plain, button]);
    w.tree.set_root(root);
    w.frame();
    Page { w, plain, button }
}

impl Page {
    fn send(&mut self, input: PointerInput, x: f32, y: f32) {
        let now = std::time::Instant::now();
        self.w.tree.pointer(input, MouseButton::Left, Point::new(x, y), Mods::NONE, now);
    }

    fn background(&self, node: NodeId) -> Color {
        self.w.tree.paint_style(node).background
    }
}

#[test]
fn hover_and_press_show_through_the_style_pass_and_cost_one_node() {
    let mut p = page();
    p.send(PointerInput::Move, 50.0, 20.0); // over the plain box: a state nothing styles
    assert!(p.w.tree.state(p.plain).contains(State::HOVER) && !p.w.tree.needs_frame());

    p.send(PointerInput::Move, 50.0, 60.0);
    let (stats, drawn) = p.w.frame();
    assert_eq!(p.background(p.button), Color::rgb(LIGHT));
    // specs/14, "Pointer enters a button": one node styled, none laid out, one painted.
    assert_eq!((stats.nodes_styled, stats.nodes_laid_out, stats.nodes_painted), (1, 0, 1));
    assert_eq!((stats.layers_patched, drawn.instances_uploaded), (1, 1));

    p.send(PointerInput::Down, 50.0, 60.0); // pressed and hovered: the later state wins
    p.w.frame();
    assert_eq!(p.background(p.button), Color::rgb(DARK));
    p.send(PointerInput::Up, 50.0, 60.0);
    p.w.frame();
    assert_eq!(p.background(p.button), Color::rgb(LIGHT));
    p.send(PointerInput::Leave, 0.0, 0.0);
    let (stats, _) = p.w.frame();
    assert_eq!((p.background(p.button), stats.nodes_styled), (Color::rgb(GRAY), 1));
}

#[test]
fn a_nodes_own_style_changes_under_its_variants() {
    let mut p = page();
    p.send(PointerInput::Move, 50.0, 60.0);
    p.w.frame();
    // Its own background changes while it is hovered: the hover colour still shows.
    p.w.tree.patch_style(p.button, Patch::Background(Color::rgb(0x112233).into()));
    assert!(!p.w.tree.needs_frame()); // resolved to the same style: nothing to do
    p.w.tree.patch_style(p.button, Patch::Width(quadra_layout::Length::Px(160.0)));
    let (stats, _) = p.w.frame();
    assert!(stats.nodes_laid_out > 0 && p.w.tree.rect(p.button).width == 160.0);
    assert_eq!(p.background(p.button), Color::rgb(LIGHT));
    p.send(PointerInput::Leave, 0.0, 0.0);
    p.w.frame();
    assert_eq!(p.background(p.button), Color::rgb(0x112233));
}

#[test]
fn a_variant_may_change_layout_and_a_state_may_be_a_property() {
    let mut p = page();
    let wider = vec![Patch::Width(quadra_layout::Length::Px(140.0)), Patch::Radius(8.0.into())];
    p.w.tree.set_variant(p.button, State::SELECTED, wider);
    p.w.tree.set_state(p.button, State::SELECTED, true);
    let (stats, _) = p.w.frame();
    assert!(stats.nodes_styled == 1 && stats.nodes_laid_out > 0);
    assert_eq!((p.w.tree.rect(p.button).width, p.background(p.button)), (140.0, Color::rgb(GRAY)));
    p.w.tree.set_state(p.button, State::SELECTED, true); // set again: nothing
    assert!(!p.w.tree.needs_frame());
    p.w.tree.set_state(p.button, State::SELECTED, false);
    p.w.frame();
    assert_eq!(p.w.tree.rect(p.button).width, 100.0);
    // A state without a variant changes nothing, on any node.
    p.w.tree.set_state(p.button, State::DISABLED, true);
    p.w.tree.set_state(p.plain, State::SELECTED, true);
    assert!(!p.w.tree.needs_frame());
}

/// Two roles, and a theme of a colour for each.
const SURFACE: Token = Token(0);
const ACCENT: Token = Token(1);

fn theme(surface: u32, accent: u32) -> Rc<Theme> {
    Rc::new(Theme::new(vec![Color::rgb(surface), Color::rgb(accent)]))
}

#[test]
fn a_role_gets_its_colour_from_the_theme_and_another_when_the_theme_changes() {
    let mut p = page();
    p.w.tree.set_theme(theme(0x202020, 0x3366FF));
    // The plain box is the surface; the button turns the accent, half as opaque, when hovered.
    p.w.tree.patch_style(p.plain, Patch::Background(SURFACE.into()));
    p.w.tree.set_variant(p.button, State::HOVER, vec![Patch::Background(ACCENT.alpha(0.5))]);
    p.w.frame();
    assert_eq!(p.background(p.plain), Color::rgb(0x202020));
    assert_eq!(p.background(p.button), Color::rgb(GRAY)); // not hovered: its own colour
    p.send(PointerInput::Move, 50.0, 60.0);
    p.w.frame();
    assert_eq!(p.background(p.button), Color::rgb(0x3366FF).alpha(0.5));

    // Another theme: the two nodes that hold roles are styled and painted again, and
    // nothing is laid out. A third box, of plain colours, is not looked at.
    p.w.tree.set_theme(theme(0xF0F0F0, 0xCC0000));
    let (stats, _) = p.w.frame();
    assert_eq!((stats.nodes_styled, stats.nodes_laid_out, stats.nodes_painted), (2, 0, 2));
    assert_eq!(p.background(p.plain), Color::rgb(0xF0F0F0));
    assert_eq!(p.background(p.button), Color::rgb(0xCC0000).alpha(0.5));
    p.w.tree.set_theme(theme(0xF0F0F0, 0xCC0000)); // the same colours: nothing to do
    assert!(!p.w.tree.needs_frame());

    // A colour given after a role takes its place: the theme has no say in it any more.
    p.w.tree.patch_style(p.plain, Patch::Background(Color::rgb(0x112233).into()));
    p.w.frame();
    p.w.tree.set_theme(theme(0x000000, 0xCC0000));
    let (stats, _) = p.w.frame();
    assert_eq!((p.background(p.plain), stats.nodes_painted), (Color::rgb(0x112233), 0));
    // A level above the window casts the shadows the theme has for it: a theme with others
    // paints it again, and lays nothing out.
    p.w.tree.patch_style(p.plain, Patch::Elevation(1));
    p.w.frame();
    assert_eq!(p.w.tree.paint_style(p.plain).shadows, 1);
    let cast = vec![vec![Cast { down: 2.0, blur: 4.0, color: Color::BLACK.alpha(0.5) }]];
    p.w.tree.set_theme(Rc::new((*theme(0x000000, 0xCC0000)).clone().with_shadows(cast)));
    let (stats, _) = p.w.frame();
    assert_eq!((stats.nodes_painted, stats.nodes_laid_out), (1, 0));
    // A role the theme has no colour for is transparent.
    p.w.tree.patch_style(p.plain, Patch::BorderColor(Token(9).into()));
    assert_eq!(p.w.tree.paint_style(p.plain).border_color, Color::TRANSPARENT);
}

#[test]
fn a_box_with_no_fill_until_hovered_is_filled_in_place() {
    let mut p = page();
    // A ghost: nothing to see at rest. It keeps a place in the layer all the same, so that
    // the pointer coming over it overwrites one instance and moves no other.
    let ghost = p.w.block(100.0, 40.0, 0);
    p.w.tree.set_paint_style(ghost, quadra_tree::PaintStyle::default());
    p.w.tree.set_variant(ghost, State::HOVER, vec![Patch::Background(Color::rgb(LIGHT).into())]);
    let root = p.w.tree.root().unwrap();
    p.w.tree.set_children(root, &[p.plain, p.button, ghost]);
    p.w.frame();
    p.send(PointerInput::Move, 50.0, 100.0);
    let (stats, drawn) = p.w.frame();
    assert_eq!(p.background(ghost), Color::rgb(LIGHT));
    assert_eq!((stats.nodes_painted, stats.layers_patched, stats.layers_rebuilt), (1, 1, 0));
    assert_eq!(drawn.instances_uploaded, 1);
}

#[test]
fn a_level_above_the_window_casts_its_shadows_under_the_box() {
    let mut p = page();
    let cast = |n| vec![Cast { down: 2.0, blur: 4.0, color: Color::BLACK.alpha(0.2) }; n];
    p.w.tree.set_theme(Rc::new((*theme(0x000000, 0xCC0000)).clone().with_shadows(vec![cast(2), cast(4)])));
    p.w.frame();
    let mut uploaded = Vec::new();
    for level in [1, 2, 0] {
        p.w.tree.patch_style(p.plain, Patch::Elevation(level));
        let (stats, drawn) = p.w.frame();
        assert_eq!((stats.nodes_laid_out, stats.nodes_painted), (0, 1)); // paint only
        uploaded.push(drawn.instances_uploaded);
    }
    // The layer is the two boxes, and under the first: two shadows at a menu's level, four
    // at a dialog's, none on the ground, where the box keeps its room (specs/07): its range
    // of five, one box and four empty, is overwritten in place.
    assert_eq!(uploaded, [4, 6, 5]);
}
