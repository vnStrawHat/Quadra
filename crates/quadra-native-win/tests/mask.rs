//! A box cut to a shape, on the software rasterizer (specs/07, "Layers"): the mask in the
//! coverage atlas, and a layer inside the box cut by it too.

use quadra_native_win::D3d11Renderer;
use quadra_prim::{Color, Size};
use quadra_render::{DeviceSize, Renderer, SurfaceTarget};
use quadra_text::FakeBackend;
use quadra_tree::{Fonts, Tree};
use quadra_ui::{div, mount_root, row};

const CIRCLE: &str = r#"<svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="5"/></svg>"#;

#[test]
fn a_box_cut_to_a_circle_cuts_what_is_in_it_too() {
    let fonts = Fonts::new(Box::new(FakeBackend::new()));
    let mut tree = Tree::new(fonts.clone());
    let mut r = D3d11Renderer::new(true).unwrap();
    let surface = r.create_surface(SurfaceTarget::Offscreen, DeviceSize { width: 220, height: 100 }).unwrap();
    // Red cut to a circle; beside it the same with a blue layer of its own (it is faded a
    // little) filling its right half.
    let blue = div().absolute().left(50.0).top(0.0).size(50.0, 100.0).bg(Color::rgb(0x0000FF)).opacity(0.999);
    let cut = || div().size(100.0, 100.0).bg(Color::rgb(0xFF0000)).clip_shape(CIRCLE);
    mount_root(&mut tree, row().gap(20.0).child(cut()).child(cut().child(blue)));
    tree.frame(Size::new(220.0, 100.0), 1.0);
    r.sync_atlas(fonts.borrow_mut().glyphs.core_mut()).unwrap();
    r.draw(surface, tree.scene_mut(), Color::WHITE).unwrap();
    let pixels = r.read_pixels(surface).unwrap();
    let at = |x: u32, y: u32| {
        let i = ((y * pixels.size.width + x) * 4) as usize;
        [pixels.rgba[i], pixels.rgba[i + 1], pixels.rgba[i + 2]]
    };
    let (red, blue, white) = ([255, 0, 0], [0, 0, 255], [255, 255, 255]);
    assert_eq!((at(50, 50), at(2, 2), at(97, 97), at(50, 1)), (red, white, white, red));
    let [r, g, _] = at(14, 14); // on the circle's edge: partly cut
    assert!(r == 255 && g > 20 && g < 235, "{:?}", at(14, 14));
    // The layer inside is cut by the mask of the box around it.
    assert_eq!((at(180, 50), at(217, 2), at(217, 50), at(125, 50)), (blue, white, blue, red));
}
