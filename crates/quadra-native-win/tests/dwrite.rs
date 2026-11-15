//! The DirectWrite backend against the fonts every Windows has: matching, shaping with
//! clusters, fallback, coverage per pixel and per subpixel, colour glyphs, fonts from memory, OpenType features, bidirectional levels,
//! and text from a view to pixels.

use std::rc::Rc;

use quadra_native_win::{D3d11Renderer, DirectWrite};
use quadra_prim::{Color, Size};
use quadra_render::{DeviceSize, Renderer, SurfaceTarget};
use quadra_text::{FaceId, FontKey, GlyphFormat, TextBackend};
use quadra_tree::{Fonts, Tree};
use quadra_view::{column, mount_root, text};

fn key(family: &str) -> FontKey {
    FontKey { family: Rc::from(family), weight: 400, italic: false }
}

fn segoe(dw: &mut DirectWrite) -> FaceId {
    dw.face(&key("Segoe UI")).expect("Segoe UI is installed")
}

#[test]
fn a_family_is_matched_and_measured_per_em() {
    let mut dw = DirectWrite::new().unwrap();
    let face = segoe(&mut dw);
    assert_eq!(dw.face(&key("Segoe UI")), Some(face)); // the same key is the same face
    assert_ne!(dw.face(&FontKey { weight: 700, ..key("Segoe UI") }), Some(face));
    assert_eq!(dw.face(&key("No Such Family 12345")), None);
    let m = dw.metrics(face);
    assert!((0.9..1.2).contains(&m.ascent) && (0.2..0.35).contains(&m.descent), "{m:?}");
    assert!(m.underline_position > 0.0 && m.underline_thickness > 0.0, "{m:?}");
    let families = dw.families();
    assert!(families.iter().any(|f| f == "Segoe UI") && families.is_sorted());
}

#[test]
fn a_word_is_shaped_into_glyphs_that_name_their_bytes() {
    let mut dw = DirectWrite::new().unwrap();
    let face = segoe(&mut dw);
    let hello = dw.shape("Hello", face, 16.0);
    assert_eq!(hello.iter().map(|g| g.cluster).collect::<Vec<_>>(), [0, 1, 2, 3, 4]);
    assert!(hello.iter().all(|g| g.face == face && g.id != 0 && g.advance > 0.0));
    assert!(hello.windows(2).all(|w| w[1].x > w[0].x));
    let width: f32 = hello.iter().map(|g| g.advance).sum();
    assert!((30.0..45.0).contains(&width), "{width}");
    let twice: f32 = dw.shape("Hello", face, 32.0).iter().map(|g| g.advance).sum();
    assert!((twice - width * 2.0).abs() < 0.5); // advances scale with the size

    // Vietnamese: "ệ" is three bytes and one glyph; the next glyph starts after it.
    let viet = dw.shape("Việt", face, 16.0);
    assert_eq!(viet.iter().map(|g| g.cluster).collect::<Vec<_>>(), [0, 1, 2, 5]);
    assert!(viet.iter().all(|g| g.face == face && g.id != 0));
    // The same letter as a base and two marks is one cluster, however many glyphs it takes.
    let marks = dw.shape("e\u{0323}\u{0302}", face, 16.0);
    assert!(!marks.is_empty() && marks.iter().all(|g| g.cluster == 0), "{marks:?}");
    assert!(dw.shape("", face, 16.0).is_empty());
}

#[test]
fn characters_the_face_lacks_come_from_fallback_faces() {
    let mut dw = DirectWrite::new().unwrap();
    let face = segoe(&mut dw);
    let mixed = dw.shape("a日本", face, 16.0);
    assert_eq!(mixed.iter().map(|g| g.cluster).collect::<Vec<_>>(), [0, 1, 4]);
    assert_eq!(mixed[0].face, face);
    assert!(mixed[1].face != face && mixed[1].face == mixed[2].face);
    assert!(mixed.iter().all(|g| g.id != 0), "{mixed:?}"); // no missing-glyph boxes
    assert!(mixed[2].x > mixed[1].x && mixed[1].x > mixed[0].x); // one pen across the faces
}

#[test]
fn a_glyph_is_coverage_per_pixel_or_per_subpixel() {
    let mut dw = DirectWrite::new().unwrap();
    let face = segoe(&mut dw);
    let glyph = |dw: &mut DirectWrite, of: &str| dw.shape(of, face, 32.0)[0].id;
    let (h, space) = (glyph(&mut dw, "H"), glyph(&mut dw, " "));

    dw.cleartype = false;
    let gray = dw.rasterize(face, h, 32.0, 0).unwrap();
    assert_eq!(gray.format, GlyphFormat::Coverage);
    assert!(
        (15..30).contains(&gray.width) && (18..30).contains(&gray.height),
        "{}x{}",
        gray.width,
        gray.height
    );
    assert!((-30..-18).contains(&gray.top), "{}", gray.top); // it stands on the baseline
    assert_eq!(gray.pixels.len(), (gray.width * gray.height) as usize);
    assert!(gray.pixels.contains(&255) && gray.pixels.contains(&0));
    let moved = dw.rasterize(face, h, 32.0, 2).unwrap(); // half a pixel to the right
    assert_ne!(moved.pixels, gray.pixels);
    assert_eq!(dw.rasterize(face, space, 32.0, 0).unwrap().width, 0); // no ink

    dw.cleartype = true;
    let stripes = dw.rasterize(face, h, 32.0, 0).unwrap();
    assert_eq!(stripes.format, GlyphFormat::Subpixel);
    assert_eq!(stripes.pixels.len(), (stripes.width * stripes.height * 4) as usize);
    assert!(stripes.pixels.chunks_exact(4).all(|p| p[3] == p[0].max(p[1]).max(p[2])));
    assert!(stripes.pixels.chunks_exact(4).any(|p| p[0] != p[2])); // an edge between stripes
}

#[test]
fn an_emoji_is_a_colour_image() {
    let mut dw = DirectWrite::new().unwrap();
    let face = segoe(&mut dw);
    let emoji = dw.shape("\u{1F600}", face, 32.0);
    assert_eq!(emoji.len(), 1);
    assert_ne!(emoji[0].face, face);
    let image = dw.rasterize(emoji[0].face, emoji[0].id, 32.0, 0).unwrap();
    assert_eq!(image.format, GlyphFormat::Color);
    assert!(image.width > 16 && image.height > 16);
    assert!(image.pixels.chunks_exact(4).any(|p| p[0] > p[2].saturating_add(40))); // yellow, not gray
    assert!(image.pixels.chunks_exact(4).all(|p| p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3])); // premultiplied
}

#[test]
fn fonts_are_loaded_from_memory_and_found_by_their_family() {
    let mut dw = DirectWrite::new().unwrap();
    let bytes = std::fs::read("C:/Windows/Fonts/consola.ttf").expect("Consolas is installed");
    assert_eq!(dw.load_fonts(&[&bytes]).unwrap(), ["Consolas"]);
    let face = dw.face(&key("Consolas")).unwrap();
    let advances: Vec<f32> = dw.shape("iW", face, 20.0).iter().map(|g| g.advance).collect();
    assert_eq!(advances[0], advances[1]); // monospace
    assert!(dw.load_fonts(&[&[]]).is_err());
    assert!(dw.load_fonts(&[b"not a font"]).is_err());
}

#[test]
fn text_goes_from_a_view_to_pixels() {
    let mut dw = DirectWrite::new().unwrap();
    dw.cleartype = false;
    let fonts = Fonts::new(Box::new(dw));
    let mut tree = Tree::new(fonts.clone());
    let mut r = D3d11Renderer::new(true).unwrap();
    let surface = r.create_surface(SurfaceTarget::Offscreen, DeviceSize { width: 160, height: 60 }).unwrap();
    mount_root(&mut tree, column().padding(10.0).child(text("Hill").size(32.0)));
    tree.frame(Size::new(160.0, 60.0), 1.0);
    r.sync_atlas(fonts.borrow_mut().glyphs.core_mut()).unwrap();
    r.draw(surface, tree.scene_mut(), Color::WHITE).unwrap();
    let pixels = r.read_pixels(surface).unwrap();
    let dark = |x: u32, y: u32| pixels.rgba[((y * 160 + x) * 4) as usize] < 0x40;
    let ink = (0..60).flat_map(|y| (0..160).map(move |x| (x, y))).filter(|&(x, y)| dark(x, y));
    let ink: Vec<(u32, u32)> = ink.collect();
    assert!(ink.len() > 150, "{} dark pixels", ink.len()); // the stems of four letters
    let (left, right) = (ink.iter().map(|p| p.0).min().unwrap(), ink.iter().map(|p| p.0).max().unwrap());
    let (top, bottom) = (ink.iter().map(|p| p.1).min().unwrap(), ink.iter().map(|p| p.1).max().unwrap());
    assert!((10..16).contains(&left) && (50..80).contains(&right), "{left}..{right}");
    assert!((14..26).contains(&top) && (36..48).contains(&bottom), "{top}..{bottom}");
}

#[test]
fn numbers_in_segoe_ui_are_composed_and_equal_what_shaping_gives() {
    let mut ts = quadra_text::TextSystem::new(Box::new(DirectWrite::new().unwrap()));
    let face = ts.face_for(&quadra_text::TextStyle::default());
    for n in 0..64 {
        ts.word(&n.to_string(), face, 13.0); // the first numbers of a face are shaped
    }
    ts.take_stats();
    for number in ["950.28", "-3,141.59", "12:30", "+7%", "1/2"] {
        let composed = ts.word(number, face, 13.0);
        let shaped = ts.backend().shape(number, face, 13.0);
        assert_eq!(composed.glyphs, shaped, "{number}");
    }
    assert_eq!(ts.take_stats().words_composed, 5);
}

#[test]
fn a_family_is_shaped_with_the_features_it_is_given() {
    use quadra_text::Feature;
    let mut dw = DirectWrite::new().unwrap();
    let face = segoe(&mut dw);
    let plain: Vec<(u16, f32)> = dw.shape("1a", face, 16.0).iter().map(|g| (g.id, g.advance)).collect();
    dw.set_features("segoe ui", &[Feature::on(b"pnum"), Feature::on(b"smcp")]);
    let featured: Vec<(u16, f32)> = dw.shape("1a", face, 16.0).iter().map(|g| (g.id, g.advance)).collect();
    assert!(featured[0].1 < plain[0].1, "a proportional one is narrower: {featured:?}");
    assert_ne!(featured[1].0, plain[1].0, "a small capital is another glyph");
    dw.set_features("Segoe UI", &[]);
    assert_eq!(dw.shape("1a", face, 16.0).iter().map(|g| (g.id, g.advance)).collect::<Vec<_>>(), plain);
}

#[test]
fn right_to_left_text_has_its_levels_and_a_word_comes_in_visual_order() {
    let mut dw = DirectWrite::new().unwrap();
    let face = segoe(&mut dw);
    let levels = dw.bidi_levels("ab שלום 12", false).unwrap();
    assert_eq!(levels.len(), "ab שלום 12".len());
    assert_eq!((levels[0], levels[3], levels[levels.len() - 1]), (0, 1, 2), "{levels:?}");
    assert_eq!(dw.bidi_levels("שלום", true).unwrap(), [1; 8]);
    // "שלום": the last letter is the first from the left, and the pen goes right.
    let word = dw.shape("שלום", face, 16.0);
    let clusters: Vec<u32> = word.iter().map(|g| g.cluster).collect();
    assert_eq!(clusters, [6, 4, 2, 0]);
    assert!(word.windows(2).all(|w| w[0].x < w[1].x), "{word:?}");
    // A number in it keeps its order, to the left of the letters before it.
    let mixed: Vec<u32> = dw.shape("אב12", face, 16.0).iter().map(|g| g.cluster).collect();
    assert_eq!(mixed, [4, 5, 2, 0]);
}
