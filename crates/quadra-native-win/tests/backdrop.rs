//! A box that blurs what is behind it, on the software rasterizer (specs/07, "Instances"): the
//! backdrop taken before the box's layer, its mips, and the box's quad sampling them.

use quadra_native_win::D3d11Renderer;
use quadra_prim::{Color, Size};
use quadra_render::{DeviceSize, Pixels, Renderer, SurfaceTarget};
use quadra_text::FakeBackend;
use quadra_tree::{Fonts, Tree};
use quadra_ui::{div, mount_root, row};

/// Red on the left half, blue on the right, and over the seam a box that blurs them: with no
/// fill, or with a white one at half alpha. Drawn twice.
fn drawn(tint: Color) -> (Pixels, Pixels) {
    let fonts = Fonts::new(Box::new(FakeBackend::new()));
    let mut tree = Tree::new(fonts.clone());
    let mut r = D3d11Renderer::new(true).unwrap();
    let surface = r.create_surface(SurfaceTarget::Offscreen, DeviceSize { width: 200, height: 60 }).unwrap();
    let glass = div().absolute().left(40.0).top(10.0).size(120.0, 40.0).bg(tint).backdrop_blur(8.0);
    let halves = row().child(div().size(100.0, 60.0).bg(Color::rgb(0xFF0000)));
    let halves = halves.child(div().size(100.0, 60.0).bg(Color::rgb(0x0000FF))).child(glass);
    mount_root(&mut tree, halves);
    let mut frame = || {
        tree.frame(Size::new(200.0, 60.0), 1.0);
        r.sync_atlas(fonts.borrow_mut().color_glyphs.core_mut()).unwrap();
        r.draw(surface, tree.scene_mut(), Color::WHITE).unwrap();
        r.read_pixels(surface).unwrap()
    };
    (frame(), frame())
}

fn at(pixels: &Pixels, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * pixels.size.width + x) * 4) as usize;
    [pixels.rgba[i], pixels.rgba[i + 1], pixels.rgba[i + 2]]
}

#[test]
fn the_seam_behind_a_blurring_box_is_soft_and_only_there() {
    let (pixels, again) = drawn(Color::TRANSPARENT);
    let [r, _, b] = at(&pixels, 100, 30);
    assert!(r > 60 && b > 60, "on the seam, both colours: {:?}", at(&pixels, 100, 30));
    let [r, _, b] = at(&pixels, 96, 30);
    assert!(r > b, "left of the seam, more red: {:?}", at(&pixels, 96, 30));
    assert_eq!(at(&pixels, 45, 30), [255, 0, 0]); // far from the seam, as it was
    assert_eq!(at(&pixels, 155, 30), [0, 0, 255]);
    // Outside the box the seam is as sharp as it was.
    assert_eq!((at(&pixels, 99, 5), at(&pixels, 100, 5)), ([255, 0, 0], [0, 0, 255]));
    assert_eq!(pixels.rgba, again.rgba); // the same each time it is drawn
}

#[test]
fn a_translucent_fill_tints_the_glass() {
    let (pixels, _) = drawn(Color::WHITE.alpha(0.5));
    assert_eq!(at(&pixels, 45, 30), [255, 128, 128]); // half white over red
    assert_eq!(at(&pixels, 45, 5), [255, 0, 0]); // above the box
}
