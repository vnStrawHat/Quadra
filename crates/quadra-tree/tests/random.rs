//! Random changes, each frame checked against a paint from scratch (the window helper does
//! it): ranges, in-place overwrites and rebuilds must always agree with the whole picture.

mod common;

use common::{Window, block_style};
use quadra_layout::{Direction, Display, Length};
use quadra_prim::{Color, Point};
use quadra_tree::{Mirror, PaintStyle, Role};

/// A small deterministic generator: the sequences are the same on every run.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n
    }
}

#[test]
fn random_changes_always_match_a_paint_from_scratch() {
    for seed in 1..=5 {
        let mut rng = Rng(seed);
        let mut w = Window::new();
        let mut boxes = Vec::new();
        let mut leaves = Vec::new();
        for _ in 0..6 {
            let kids = [w.block(40.0, 10.0, 0x336699), w.text("label"), w.block(20.0, 6.0, 0x993366)];
            leaves.extend(kids);
            boxes.push(w.column(&kids));
        }
        let root = w.column(&boxes);
        w.tree.set_root(root);
        w.frame();
        // A listener follows along: what it is sent must always add up to the semantic tree.
        const ROLES: [Role; 4] = [Role::None, Role::Group, Role::Button, Role::ListItem];
        let mut listener = Mirror::default();
        w.tree.set_active(true);
        w.tree.listen(true);
        for step in 0..200 {
            let b = boxes[rng.below(boxes.len())];
            let leaf = leaves[rng.below(leaves.len())];
            match rng.below(15) {
                0 => {
                    let background = Color::rgb(rng.below(0xFFFFFF) as u32);
                    w.tree.set_paint_style(leaf, PaintStyle { background, ..PaintStyle::default() });
                }
                1 => w.tree.set_text(leaf, &"x".repeat(1 + rng.below(20))), // nothing on a block
                2 => {
                    let (width, height) =
                        (10.0 + rng.below(8) as f32 * 10.0, 4.0 + rng.below(5) as f32 * 4.0);
                    w.tree.set_layout_style(leaf, block_style(width, height, 0).layout);
                }
                3 => {
                    let new = if rng.below(2) == 0 { w.block(30.0, 8.0, 0x112233) } else { w.text("new") };
                    let mut kids = w.tree.children(b).to_vec();
                    kids.insert(rng.below(kids.len() + 1), new);
                    w.tree.set_children(b, &kids);
                    leaves.push(new);
                }
                4 if leaves.len() > 6 => {
                    let gone = leaves.swap_remove(rng.below(leaves.len()));
                    w.tree.remove(gone);
                }
                5 => {
                    let mut kids = w.tree.children(b).to_vec();
                    kids.reverse();
                    w.tree.set_children(b, &kids);
                }
                6 => {
                    let display = if rng.below(3) == 0 { Display::None } else { Display::Flex };
                    let style = quadra_layout::Style { display, ..w.tree.layout_style(b).clone() };
                    w.tree.set_layout_style(b, style);
                }
                8 => {
                    if w.tree.is_layer(b) {
                        w.tree.remove_layer(b);
                    } else {
                        w.tree.make_layer(b, rng.below(2) == 0);
                    }
                }
                9 => {
                    let offset = Point::new(rng.below(40) as f32 - 20.0, rng.below(40) as f32 * 0.5);
                    w.tree.set_layer_offset(b, offset); // nothing on a box that is no layer
                }
                11 => w.tree.update_semantics(b, |says| says.role = ROLES[rng.below(4)]),
                12 => w.tree.update_semantics(leaf, |says| says.role = ROLES[rng.below(4)]),
                13 => {
                    w.tree.set_focusable(leaf, true);
                    w.tree.focus(leaf);
                }
                14 => {
                    // The last of a box leaves it and is in no window, until it moves to one.
                    let kids = w.tree.children(b).to_vec();
                    w.tree.set_children(b, &kids[..kids.len().saturating_sub(1)]);
                }
                7 => {
                    // A leaf moves to another box.
                    let mut kids = w.tree.children(b).to_vec();
                    if !kids.contains(&leaf) {
                        kids.push(leaf);
                        w.tree.set_children(b, &kids);
                    }
                }
                _ => {
                    let direction = if rng.below(2) == 0 { Direction::Row } else { Direction::Column };
                    let width = if rng.below(2) == 0 { Length::Auto } else { Length::Px(300.0) };
                    let style = quadra_layout::Style { direction, width, ..w.tree.layout_style(b).clone() };
                    w.tree.set_layout_style(b, style);
                }
            }
            if w.tree.needs_frame() {
                let before = w.tree.len();
                w.frame();
                assert_eq!(w.tree.len(), before, "seed {seed} step {step}");
            }
            if let Some(update) = w.tree.take_semantic_update() {
                listener.apply(update).unwrap_or_else(|e| panic!("seed {seed} step {step}: {e}"));
            }
            let mut tree = w.tree.semantic_tree().nodes;
            tree.sort_by_key(|n| n.id);
            assert_eq!(listener.nodes(), tree, "seed {seed} step {step}");
        }
    }
}
