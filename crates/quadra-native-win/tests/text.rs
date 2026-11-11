//! From a view to pixels: signals, the tree, glyph tiles and the D3D11 renderer together, on
//! the software rasterizer with the fake text backend (a glyph is a solid box of half an em
//! by 0.7 em).

use quadra_native_win::D3d11Renderer;
use quadra_prim::{Color, Size};
use quadra_reactive::signal;
use quadra_render::{DeviceSize, Pixels, Renderer, SurfaceTarget};
use quadra_text::FakeBackend;
use quadra_tree::{Fonts, Tree};
use quadra_ui::{column, div, mount_root, text};

fn at(pixels: &Pixels, x: u32, y: u32) -> u32 {
    let i = ((y * pixels.size.width + x) * 4) as usize;
    (pixels.rgba[i] as u32) << 16 | (pixels.rgba[i + 1] as u32) << 8 | pixels.rgba[i + 2] as u32
}

#[test]
fn text_is_drawn_from_glyph_tiles_and_follows_its_signal() {
    let label = signal("ab".to_string());
    let fonts = Fonts::new(Box::new(FakeBackend::new()));
    let mut tree = Tree::new(fonts.clone());
    let mut r = D3d11Renderer::new(true).unwrap();
    let surface = r.create_surface(SurfaceTarget::Offscreen, DeviceSize { width: 96, height: 64 }).unwrap();
    // A green card 8 px from the corner, with text at size 20: glyphs are 10 x 14 boxes, 12 px
    // apart, standing on a baseline 22 px down the 28 px line.
    let card = div().size(60.0, 28.0).bg(Color::rgb(0x00FF00)).child(text(label).size(20.0));
    mount_root(&mut tree, column().padding(8.0).child(card));

    let frame = |tree: &mut Tree, r: &mut D3d11Renderer| {
        tree.frame(Size::new(96.0, 64.0), 1.0);
        r.sync_atlas(fonts.borrow_mut().glyphs.core_mut()).unwrap();
        r.draw(surface, tree.scene_mut(), Color::WHITE).unwrap();
        r.read_pixels(surface).unwrap()
    };
    let pixels = frame(&mut tree, &mut r);
    assert_eq!(at(&pixels, 4, 4), 0xFFFFFF); // the padding
    assert_eq!(at(&pixels, 60, 12), 0x00FF00); // the card, right of the text
    assert_eq!((at(&pixels, 12, 24), at(&pixels, 24, 24)), (0x000000, 0x000000)); // "a" and "b"
    assert_eq!(at(&pixels, 19, 24), 0x00FF00); // between them
    assert_eq!(at(&pixels, 12, 12), 0x00FF00); // above the glyphs, in the line box

    label.set("a".to_string()); // one glyph fewer
    let pixels = frame(&mut tree, &mut r);
    assert_eq!((at(&pixels, 12, 24), at(&pixels, 24, 24)), (0x000000, 0x00FF00));
    assert_eq!(fonts.borrow().glyphs.stats().tiles, 2); // "a" and "b": each rasterized once
}
