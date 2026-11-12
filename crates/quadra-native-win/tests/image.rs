//! The system's codecs behind `ImageDecoder` (specs/10): a file's size from its header, its
//! pixels premultiplied and in the order the renderer takes, at the size asked for.

use quadra_layout::Align;
use quadra_native_win::{D3d11Renderer, WicImages};
use quadra_platform::{ImageDecoder, ImageError, ImageInfo};
use quadra_prim::{Color, Size};
use quadra_render::{DeviceSize, Pixels, Renderer, SurfaceTarget};
use quadra_text::FakeBackend;
use quadra_tree::{Fit, Fonts, Image, Picture, Tree};
use quadra_ui::{column, image, mount_root};

/// A bitmap file of 24 bits a pixel: rows from the bottom up, blue first, padded to four
/// bytes.
fn bmp(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    let row = (width as usize * 3).next_multiple_of(4);
    let mut file = Vec::from(*b"BM");
    let words = |file: &mut Vec<u8>, words: &[u32]| words.iter().for_each(|w| file.extend(w.to_le_bytes()));
    words(&mut file, &[54 + (row * height as usize) as u32, 0, 54, 40, width, height]);
    file.extend([1, 0, 24, 0]); // one plane, 24 bits
    words(&mut file, &[0, 0, 2835, 2835, 0, 0]);
    for y in (0..height).rev() {
        let start = file.len();
        for x in 0..width {
            let [r, g, b] = pixel(x, y);
            file.extend([b, g, r]);
        }
        file.resize(start + row, 0);
    }
    file
}

#[test]
fn a_file_is_measured_decoded_and_scaled_by_the_systems_codecs() {
    // Red on the left, blue on the right.
    let file = bmp(4, 2, |x, _| if x < 2 { [255, 0, 0] } else { [0, 0, 255] });
    assert_eq!(WicImages.info(&file), Ok(ImageInfo { width: 4, height: 2 }));

    let whole = WicImages.decode(&file, 4, 2).unwrap();
    assert_eq!((whole.width, whole.height, whole.pixels.len()), (4, 2, 32));
    assert_eq!(whole.pixels[..4], [255, 0, 0, 255]);
    assert_eq!(whole.pixels[12..16], [0, 0, 255, 255]);

    let half = WicImages.decode(&file, 2, 1).unwrap();
    assert_eq!(half.pixels, [255, 0, 0, 255, 0, 0, 255, 255]);

    // What is no image is an error, from a thread of any kind and with no panic.
    assert!(matches!(WicImages.info(b"not an image at all"), Err(ImageError::Corrupt(_))));
    let cut = &file[..40];
    assert!(
        std::thread::spawn({
            let cut = cut.to_vec();
            move || WicImages.decode(&cut, 4, 2).is_err()
        })
        .join()
        .unwrap()
    );
}

fn at(pixels: &Pixels, x: u32, y: u32) -> u32 {
    let i = ((y * pixels.size.width + x) * 4) as usize;
    (pixels.rgba[i] as u32) << 16 | (pixels.rgba[i + 1] as u32) << 8 | pixels.rgba[i + 2] as u32
}

/// A picture of 600 by 4 pixels, red then blue, in a box of a width, drawn on the software
/// rasterizer: from the file through the system's codec, the pieces of the colour atlas and
/// the renderer to pixels.
fn drawn(width: f32, height: f32) -> Pixels {
    let file = bmp(600, 4, |x, _| if x < 300 { [255, 0, 0] } else { [0, 0, 255] });
    let decoded = WicImages.decode(&file, 600, 4).unwrap();
    let fonts = Fonts::new(Box::new(FakeBackend::new()));
    let mut tree = Tree::new(fonts.clone());
    let mut r = D3d11Renderer::new(true).unwrap();
    let surface = r.create_surface(SurfaceTarget::Offscreen, DeviceSize { width: 640, height: 16 }).unwrap();
    // Nobody decodes for a view mounted with no app around: the test gives it its pixels.
    let view = image(Vec::new()).width(width).height(height).fit(Fit::Fill);
    mount_root(&mut tree, column().align_items(Align::Start).child(view));
    let id = tree.children(tree.root().unwrap())[0];
    let picture = tree.widget_mut::<Picture>(id).unwrap();
    picture.set_natural(Size::new(600.0, 4.0));
    picture.set_image(Image::new(decoded));
    tree.frame(Size::new(640.0, 16.0), 1.0);
    r.sync_atlas(fonts.borrow_mut().color_glyphs.core_mut()).unwrap();
    r.draw(surface, tree.scene_mut(), Color::WHITE).unwrap();
    r.read_pixels(surface).unwrap()
}

#[test]
fn a_picture_is_drawn_from_pieces_that_meet_at_its_own_size_and_at_another() {
    const RED: u32 = 0xFF0000;
    const BLUE: u32 = 0x0000FF;
    // A texel a pixel: the colours change where the file's do, and the two pieces (the
    // second starts at 512) meet with nothing between them.
    let pixels = drawn(600.0, 4.0);
    assert_eq!((at(&pixels, 0, 0), at(&pixels, 299, 2), at(&pixels, 300, 2)), (RED, RED, BLUE));
    assert_eq!((at(&pixels, 511, 2), at(&pixels, 512, 2), at(&pixels, 599, 3)), (BLUE, BLUE, BLUE));
    assert_eq!((at(&pixels, 600, 2), at(&pixels, 10, 4)), (0xFFFFFF, 0xFFFFFF));

    // Half the size from the same pixels, as while a new size is being decoded: still no
    // seam where the pieces meet, at 256.
    let pixels = drawn(300.0, 2.0);
    assert_eq!((at(&pixels, 148, 1), at(&pixels, 151, 1)), (RED, BLUE));
    assert_eq!((at(&pixels, 255, 1), at(&pixels, 256, 1), at(&pixels, 299, 1)), (BLUE, BLUE, BLUE));
    assert_eq!(at(&pixels, 300, 1), 0xFFFFFF);
}
