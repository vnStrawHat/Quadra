//! The Core Text backend against the fonts every Mac has: matching, the system's own font,
//! shaping with clusters, fallback and the families tried before it, coverage a pixel, colour
//! glyphs, fonts from memory, OpenType features, bidirectional levels, and numbers composed as
//! the text system composes them.
#![cfg(target_os = "macos")]

use std::rc::Rc;

use quadra_native_mac::{CoreText, MetalRenderer, SYSTEM_FAMILY};
use quadra_text::{FaceId, FontKey, GlyphFormat, TextBackend};

fn key(family: &str) -> FontKey {
    FontKey { family: Rc::from(family), weight: 400, italic: false }
}

fn helvetica(ct: &mut CoreText) -> FaceId {
    ct.face(&key("Helvetica Neue")).expect("Helvetica Neue is installed")
}

fn width(ct: &mut CoreText, text: &str, face: FaceId, size: f32) -> f32 {
    ct.shape(text, face, size).iter().map(|g| g.advance).sum()
}

#[test]
fn a_family_is_matched_and_measured_per_em() {
    let mut ct = CoreText::new().unwrap();
    let face = helvetica(&mut ct);
    assert_eq!(ct.face(&key("Helvetica Neue")), Some(face)); // the same key is the same face
    assert_eq!(ct.face(&key("helvetica neue")).map(|f| ct.metrics(f)), Some(ct.metrics(face)));
    let bold = ct.face(&FontKey { weight: 700, ..key("Helvetica Neue") }).unwrap();
    assert_ne!(bold, face);
    assert!(width(&mut ct, "Hello", bold, 16.0) > width(&mut ct, "Hello", face, 16.0), "bold is wider");
    let italic = ct.face(&FontKey { italic: true, ..key("Helvetica Neue") }).unwrap();
    assert_ne!(ct.shape("a", italic, 16.0)[0].id, 0);
    assert_eq!(ct.face(&key("No Such Family 12345")), None);
    let m = ct.metrics(face);
    assert!((0.7..1.2).contains(&m.ascent) && (0.15..0.35).contains(&m.descent), "{m:?}");
    assert!(m.underline_position > 0.0 && m.underline_thickness > 0.0, "{m:?}");
    let families = ct.families();
    assert!(families.iter().any(|f| f == "Helvetica Neue") && families.is_sorted());
    assert!(families.iter().any(|f| f == SYSTEM_FAMILY));
}

#[test]
fn the_system_s_own_font_is_the_default_family_at_every_weight() {
    let mut ct = CoreText::new().unwrap();
    assert_eq!(ct.default_family(), SYSTEM_FAMILY);
    let regular = ct.face(&key(SYSTEM_FAMILY)).expect("the system has a font of its own");
    let bold = ct.face(&FontKey { weight: 700, ..key(SYSTEM_FAMILY) }).unwrap();
    let hello = ct.shape("Hello", regular, 16.0);
    assert_eq!(hello.len(), 5);
    assert!(hello.iter().all(|g| g.face == regular && g.id != 0 && g.advance > 0.0), "{hello:?}");
    let (thin, thick) = (width(&mut ct, "Hello", regular, 16.0), width(&mut ct, "Hello", bold, 16.0));
    assert!(thick > thin, "bold {thick} is wider than regular {thin}");
    let m = ct.metrics(regular);
    assert!((0.7..1.2).contains(&m.ascent) && (0.15..0.35).contains(&m.descent), "{m:?}");
}

#[test]
fn a_word_is_shaped_into_glyphs_that_name_their_bytes() {
    let mut ct = CoreText::new().unwrap();
    let face = helvetica(&mut ct);
    let hello = ct.shape("Hello", face, 16.0);
    assert_eq!(hello.iter().map(|g| g.cluster).collect::<Vec<_>>(), [0, 1, 2, 3, 4]);
    assert!(hello.iter().all(|g| g.face == face && g.id != 0 && g.advance > 0.0));
    assert!(hello.windows(2).all(|w| w[1].x > w[0].x));
    assert_eq!(hello[0].x, 0.0);
    let wide: f32 = hello.iter().map(|g| g.advance).sum();
    assert!((30.0..45.0).contains(&wide), "{wide}");
    let twice = width(&mut ct, "Hello", face, 32.0);
    assert!((twice - wide * 2.0).abs() < 0.5); // advances scale with the size

    // Vietnamese: "ệ" is three bytes and one glyph; the next glyph starts after it.
    let viet = ct.shape("Việt", face, 16.0);
    assert_eq!(viet.iter().map(|g| g.cluster).collect::<Vec<_>>(), [0, 1, 2, 5], "{viet:?}");
    assert!(viet.iter().all(|g| g.id != 0), "{viet:?}");
    // The same letter as a base and two marks is one cluster, however many glyphs it takes.
    let marks = ct.shape("e\u{0323}\u{0302}", face, 16.0);
    assert!(!marks.is_empty() && marks.iter().all(|g| g.cluster == 0), "{marks:?}");
    assert!(ct.shape("", face, 16.0).is_empty());
}

#[test]
fn characters_the_face_lacks_come_from_fallback_faces() {
    let mut ct = CoreText::new().unwrap();
    let face = helvetica(&mut ct);
    let mixed = ct.shape("a日本", face, 16.0);
    assert_eq!(mixed.iter().map(|g| g.cluster).collect::<Vec<_>>(), [0, 1, 4], "{mixed:?}");
    assert_eq!(mixed[0].face, face);
    assert!(mixed[1].face != face && mixed[1].face == mixed[2].face, "{mixed:?}");
    assert!(mixed.iter().all(|g| g.id != 0), "{mixed:?}"); // no missing-glyph boxes
    assert!(mixed[2].x > mixed[1].x && mixed[1].x > mixed[0].x); // one pen across the faces
    // The face fallback chose is a face as any other: measured, and the same the next time.
    assert!(ct.metrics(mixed[1].face).ascent > 0.5);
    assert_eq!(ct.shape("本", face, 16.0)[0].face, mixed[1].face);
    // Right to left: the last letter is the first from the left, and the pen goes right.
    let word = ct.shape("שלום", face, 16.0);
    assert_eq!(word.iter().map(|g| g.cluster).collect::<Vec<_>>(), [6, 4, 2, 0], "{word:?}");
    assert!(word.windows(2).all(|w| w[0].x < w[1].x), "{word:?}");
}

#[test]
fn a_glyph_is_coverage_a_pixel() {
    let mut ct = CoreText::new().unwrap();
    let face = helvetica(&mut ct);
    let glyph = |ct: &mut CoreText, of: &str| ct.shape(of, face, 32.0)[0].id;
    let (h, space) = (glyph(&mut ct, "H"), glyph(&mut ct, " "));
    let gray = ct.rasterize(face, h, 32.0, 0).unwrap();
    assert_eq!(gray.format, GlyphFormat::Coverage);
    assert!(
        (15..34).contains(&gray.width) && (18..34).contains(&gray.height),
        "{}x{}",
        gray.width,
        gray.height
    );
    assert!((-32..-18).contains(&gray.top), "{}", gray.top); // it stands on the baseline
    assert_eq!(gray.pixels.len(), (gray.width * gray.height) as usize);
    assert!(gray.pixels.contains(&255) && gray.pixels.contains(&0));
    // The top row is the pixel kept for smoothing: nothing.
    assert!(gray.pixels[..gray.width as usize].iter().all(|&p| p == 0));
    // An L says which way up and which way round the bitmap is: its stem alone near the top,
    // at the left; its bar near the bottom, most of the way across.
    let l = glyph(&mut ct, "L");
    let l = ct.rasterize(face, l, 32.0, 0).unwrap();
    let inked = |y: u32| {
        let row = &l.pixels[(y * l.width) as usize..((y + 1) * l.width) as usize];
        (0..l.width).filter(|&x| row[x as usize] > 128).collect::<Vec<u32>>()
    };
    let (stem, bar) = (inked(l.height / 4), inked(l.height - 4));
    assert!(!stem.is_empty() && stem.iter().all(|&x| x < l.width / 2), "the stem: {stem:?} of {}", l.width);
    assert!(bar.len() as u32 > l.width / 2 && bar.len() > stem.len() * 2, "the bar: {bar:?} of {}", l.width);
    let moved = ct.rasterize(face, h, 32.0, 2).unwrap(); // half a pixel to the right
    assert_ne!(moved.pixels, gray.pixels);
    assert_eq!(ct.rasterize(face, space, 32.0, 0).unwrap().width, 0); // no ink
}

#[test]
fn an_emoji_is_a_colour_image() {
    let mut ct = CoreText::new().unwrap();
    let face = helvetica(&mut ct);
    let emoji = ct.shape("\u{1F600}", face, 32.0);
    assert_eq!(emoji.len(), 1, "{emoji:?}");
    assert_ne!(emoji[0].face, face);
    assert_eq!(emoji[0].cluster, 0);
    let image = ct.rasterize(emoji[0].face, emoji[0].id, 32.0, 0).unwrap();
    assert_eq!(image.format, GlyphFormat::Color);
    assert!(image.width > 16 && image.height > 16, "{}x{}", image.width, image.height);
    assert_eq!(image.pixels.len(), (image.width * image.height * 4) as usize);
    assert!(image.pixels.chunks_exact(4).any(|p| p[0] > p[2].saturating_add(40))); // yellow, not gray
    assert!(image.pixels.chunks_exact(4).all(|p| p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3])); // premultiplied
}

#[test]
fn fonts_are_loaded_from_memory_and_found_by_their_family() {
    let mut ct = CoreText::new().unwrap();
    // A collection: the four faces of one family.
    let bytes = std::fs::read("/System/Library/Fonts/Menlo.ttc").expect("Menlo is installed");
    assert_eq!(ct.load_fonts(&[&bytes]).unwrap(), ["Menlo"]);
    let face = ct.face(&key("Menlo")).unwrap();
    let advances: Vec<f32> = ct.shape("iW", face, 20.0).iter().map(|g| g.advance).collect();
    assert_eq!(advances[0], advances[1]); // monospace
    let bold = ct.face(&FontKey { weight: 700, ..key("Menlo") }).unwrap();
    let italic = ct.face(&FontKey { italic: true, ..key("Menlo") }).unwrap();
    assert!(bold != face && italic != face && italic != bold);
    let ink = |ct: &mut CoreText, face: FaceId| {
        let glyph = ct.shape("l", face, 32.0)[0].id;
        ct.rasterize(face, glyph, 32.0, 0).unwrap().pixels.iter().map(|&p| p as u32).sum::<u32>()
    };
    assert!(ink(&mut ct, bold) > ink(&mut ct, face), "the bold of the family has more ink");
    assert!(ct.load_fonts(&[&[]]).is_err());
    assert!(ct.load_fonts(&[b"not a font"]).is_err());
}

#[test]
fn numbers_are_composed_and_equal_what_shaping_gives() {
    // The system's font, whose numbers may not compose (a colon between digits is raised),
    // and one whose numbers do: either way a number is what shaping gives.
    for family in [SYSTEM_FAMILY, "Menlo"] {
        let mut ts = quadra_text::TextSystem::new(Box::new(CoreText::new().unwrap()));
        let style = quadra_text::TextStyle { family: Rc::from(family), ..Default::default() };
        let face = ts.face_for(&style);
        for n in 0..64 {
            ts.word(&n.to_string(), face, 13.0); // the first numbers of a face are shaped
        }
        for number in ["950.28", "-3,141.59", "12:30", "+7%", "1/2"] {
            let composed = ts.word(number, face, 13.0);
            let shaped = ts.backend().shape(number, face, 13.0);
            assert_eq!(composed.glyphs, shaped, "{family}: {number}");
        }
    }
}

#[test]
fn the_families_named_are_tried_before_the_system_fallback() {
    let mut ct = CoreText::new().unwrap();
    let face = helvetica(&mut ct);
    let first = |ct: &mut CoreText| ct.shape("\u{65E5}", face, 16.0)[0];
    let system = first(&mut ct);
    let mut chosen = Vec::new();
    for family in ["Arial Unicode MS", "Hiragino Sans"] {
        ct.set_fallbacks(&["No Such Family 12345".to_string(), family.to_string()]);
        let named = ct.face(&key(family)).unwrap_or_else(|| panic!("{family} is installed"));
        let (direct, through) = (ct.shape("\u{65E5}", named, 16.0)[0], first(&mut ct));
        assert_eq!((through.id, through.advance), (direct.id, direct.advance), "{family}");
        assert_ne!(through.face, face);
        chosen.push(through.face);
    }
    assert_ne!(chosen[0], chosen[1], "each family named is the one that is drawn in");
    ct.set_fallbacks(&[]);
    assert_eq!(first(&mut ct), system);
}

#[test]
fn a_family_is_shaped_with_the_features_it_is_given() {
    use quadra_text::Feature;
    let mut ct = CoreText::new().unwrap();
    let face = ct.face(&key(SYSTEM_FAMILY)).unwrap();
    let shaped = |ct: &mut CoreText| -> Vec<(u16, f32)> {
        ct.shape("1a", face, 16.0).iter().map(|g| (g.id, g.advance)).collect()
    };
    let plain = shaped(&mut ct);
    ct.set_features("System-UI", &[Feature::on(b"smcp")]);
    let small = shaped(&mut ct);
    assert_ne!(small[1].0, plain[1].0, "a small capital is another glyph: {small:?} {plain:?}");
    assert_eq!(small[0], plain[0], "a digit is as it was");
    // Features of another family are not this one's.
    ct.set_features(SYSTEM_FAMILY, &[]);
    ct.set_features("Helvetica Neue", &[Feature::on(b"smcp")]);
    assert_eq!(shaped(&mut ct), plain);
}

#[test]
fn right_to_left_text_has_its_levels_and_a_word_comes_in_visual_order() {
    let mut ct = CoreText::new().unwrap();
    let face = helvetica(&mut ct);
    let levels = ct.bidi_levels("ab שלום 12", false).unwrap();
    assert_eq!(levels.len(), "ab שלום 12".len());
    assert_eq!((levels[0], levels[3], levels[levels.len() - 1]), (0, 1, 2), "{levels:?}");
    assert_eq!(ct.bidi_levels("שלום", true).unwrap(), [1; 8]);
    assert_eq!(ct.bidi_levels("abc", false).unwrap(), [0; 3]);
    assert_eq!(ct.bidi_levels("", false).unwrap(), []);
    // A number in a word keeps its order, to the left of the letters before it.
    let mixed: Vec<u32> = ct.shape("אב12", face, 16.0).iter().map(|g| g.cluster).collect();
    assert_eq!(mixed, [4, 5, 2, 0]);
}

#[test]
fn text_goes_from_a_view_to_pixels() {
    use quadra_prim::{Color, Size};
    use quadra_render::{DeviceSize, Renderer, SurfaceTarget};
    use quadra_tree::{Fonts, Tree};
    use quadra_view::{column, mount_root, text};
    let fonts = Fonts::new(Box::new(CoreText::new().unwrap()));
    let mut tree = Tree::new(fonts.clone());
    let mut r = MetalRenderer::new().unwrap();
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
    assert!((10..18).contains(&left) && (45..85).contains(&right), "{left}..{right}");
    assert!((10..28).contains(&top) && (34..50).contains(&bottom), "{top}..{bottom}");
}
