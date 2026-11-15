//! Gradients on the software rasterizer (specs/07, "Instances"): from a box's gradient through
//! its ramp in the colour atlas and the shader to pixels.

use quadra_layout::{Align, Direction};
use quadra_native_win::D3d11Renderer;
use quadra_prim::{Color, Size};
use quadra_render::{DeviceSize, Pixels, Renderer, SurfaceTarget};
use quadra_text::FakeBackend;
use quadra_tree::{Fonts, Gradient, Tree};
use quadra_view::{View, div, mount_root, row};

const RED: Color = Color::rgb(0xFF0000);
const BLUE: Color = Color::rgb(0x0000FF);

/// Boxes of 100 by 100 side by side, drawn.
fn drawn(boxes: impl View + 'static) -> Pixels {
    let fonts = Fonts::new(Box::new(FakeBackend::new()));
    let mut tree = Tree::new(fonts.clone());
    let mut r = D3d11Renderer::new(true).unwrap();
    let surface = r.create_surface(SurfaceTarget::Offscreen, DeviceSize { width: 400, height: 100 }).unwrap();
    mount_root(&mut tree, row().direction(Direction::Row).align_items(Align::Start).child(boxes));
    tree.frame(Size::new(400.0, 100.0), 1.0);
    r.sync_atlas(fonts.borrow_mut().color_glyphs.core_mut()).unwrap();
    r.draw(surface, tree.scene_mut(), Color::WHITE).unwrap();
    r.read_pixels(surface).unwrap()
}

fn at(pixels: &Pixels, x: u32, y: u32) -> Color {
    let i = ((y * pixels.size.width + x) * 4) as usize;
    let c = |k: usize| pixels.rgba[i + k] as f32 / 255.0;
    Color { r: c(0), g: c(1), b: c(2), a: 1.0 }
}

/// Within two levels of a byte on each channel.
fn near(a: Color, b: Color) -> bool {
    [(a.r, b.r), (a.g, b.g), (a.b, b.b)].iter().all(|(x, y)| (x - y).abs() <= 2.5 / 255.0)
}

#[test]
fn a_gradient_runs_through_its_stops_in_oklab_each_way() {
    let boxed = |g: Gradient| div().size(100.0, 100.0).gradient(g);
    let pixels = drawn(
        row()
            .child(boxed(Gradient::linear(0.0, [RED, BLUE])))
            .child(boxed(Gradient::radial([RED, BLUE])))
            .child(boxed(Gradient::conic(0.0, [RED, BLUE])))
            .child(boxed(Gradient::linear(90.0, [RED, BLUE]).stops([(0.0, RED), (0.5, BLUE), (1.0, RED)]))),
    );
    let check = |x: u32, y: u32, want: Color| {
        let got = at(&pixels, x, y);
        assert!(near(got, want), "({x}, {y}): {} where {} was wanted", got.hex(), want.hex());
    };
    let g = Gradient::linear(0.0, [RED, BLUE]);
    // Linear, left to right: the ends, and a violet in the middle, not the grey of encoded values.
    check(0, 50, g.at(0.005));
    check(99, 50, g.at(0.995));
    check(50, 50, g.at(0.505));
    // Radial: red in the middle, blue in the corners, halfway along the way to one.
    check(150, 50, g.at(0.01)); // pixels are sampled at their middles
    check(100, 0, g.at(0.99));
    check(125, 25, g.at(0.49));
    // Conic from the top, clockwise: red just right of the top, a quarter round at the right,
    // half round at the bottom.
    check(251, 1, g.at(0.005));
    check(299, 50, g.at(0.2516));
    check(250, 99, g.at(0.4984));
    // Three stops down: blue in the middle row, red at both ends.
    check(350, 0, g.at(0.01));
    check(350, 50, g.at(0.99));
    check(350, 99, g.at(0.01));
}
