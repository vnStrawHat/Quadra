//! The image view (specs/11, "Images"): its size is its file's until another is given, its
//! pixels are decoded on the pool at the size it is shown at and painted from pieces of the
//! colour atlas, a file that is no image leaves the box empty, and a view that comes back
//! shows at once. The harness decodes flat test images (`FlatImages`).

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use quadra_layout::Align;
use quadra_platform::{FlatImages, ImageError};
use quadra_prim::{Rect, Size};
use quadra_reactive::signal;
use quadra_render::{Instance, Kind};
use quadra_testing::Harness;
use quadra_tree::{Fit, NodeId, Picture, Role};
use quadra_ui::{ImageSource, column, image, row, scroll, show};

/// The pieces of pictures that were painted.
fn pieces(h: &Harness) -> Vec<Instance> {
    let layer = h.tree.scene().layer(h.tree.scene().roots()[0]);
    layer.instances().iter().filter(|i| i.kind == Kind::Poly as u32).copied().collect()
}

fn first(h: &Harness) -> NodeId {
    h.tree.children(h.tree.root().unwrap())[0]
}

fn file(width: u32, height: u32) -> &'static [u8] {
    Box::leak(FlatImages::file(width, height, 0xff00_00ff).into_boxed_slice())
}

#[test]
fn an_image_is_as_large_as_its_file_says_until_a_size_is_given() {
    let mut h = Harness::new();
    let loaded = Rc::new(RefCell::new(None));
    let told = loaded.clone();
    h.mount(move || {
        let picture =
            image(file(200, 100)).label("A red field").on_load(move |r| *told.borrow_mut() = Some(r));
        column().align_items(Align::Start).child(picture)
    });
    // Nothing is known in the first frame; then the header, then the pixels.
    h.frame();
    assert!(pieces(&h).is_empty() && h.tree.rect(first(&h)).size() == Size::ZERO);
    h.settle();
    assert_eq!(*loaded.borrow(), Some(Ok(Size::new(200.0, 100.0))));
    assert_eq!(h.tree.rect(first(&h)), Rect::new(0.0, 0.0, 200.0, 100.0));
    let painted = pieces(&h);
    assert_eq!(painted.len(), 1);
    assert_eq!(painted[0].bounds, [0.0, 0.0, 200.0, 100.0]);
    // Its one piece is in the atlas: the pixels are let go of.
    assert_eq!(h.images.held(), (1, 0));
    assert_eq!(h.get_by_role(Role::Image, "A red field").node(), first(&h));

    // A width: the height follows the file's shape, and the pixels are decoded that size.
    let mut h = Harness::new();
    h.mount(|| column().align_items(Align::Start).child(image(file(200, 100)).width(50.0)));
    h.settle();
    assert_eq!(h.tree.rect(first(&h)).size(), Size::new(50.0, 25.0));
    let picture = h.tree.widget_mut::<Picture>(first(&h)).unwrap().image().unwrap().clone();
    assert_eq!((picture.width, picture.height, h.images.held().0), (50, 25, 1));

    // No wider than there is room, in its own shape.
    let mut h = Harness::new();
    h.mount(|| column().width(120.0).align_items(Align::Start).child(image(file(240, 60))));
    h.settle();
    assert_eq!(h.tree.rect(first(&h)).size(), Size::new(120.0, 30.0));

    // At twice the scale it wants twice the pixels, and never more than the file has.
    let mut h = Harness::new();
    h.scale = 2.0;
    h.mount(|| {
        column()
            .align_items(Align::Start)
            .child(image(file(200, 100)).width(80.0))
            .child(image(file(30, 30)).width(60.0))
    });
    h.settle();
    let size = |h: &mut Harness, at: usize| {
        let id = h.tree.children(h.tree.root().unwrap())[at];
        let image = h.tree.widget_mut::<Picture>(id).unwrap().image().unwrap().clone();
        (image.width, image.height)
    };
    assert_eq!((size(&mut h, 0), size(&mut h, 1), h.images.held().0), ((160, 80), (30, 30), 2));
    assert_eq!(pieces(&h)[1].bounds, [0.0, 80.0, 120.0, 120.0]);
}

#[test]
fn a_box_of_another_shape_is_fitted_covered_or_filled() {
    let shown = |fit: Fit| {
        let mut h = Harness::new();
        h.mount(move || row().child(image(file(200, 100)).width(100.0).height(100.0).fit(fit).radius(8.0)));
        h.settle();
        let piece = pieces(&h)[0];
        (piece.bounds, piece.clip)
    };
    // All of it in the middle; over the box and cut to it, with its corners; stretched.
    assert_eq!(shown(Fit::Contain).0, [0.0, 25.0, 100.0, 50.0]);
    assert_eq!(shown(Fit::Cover), ([-50.0, 0.0, 200.0, 100.0], [0.0, 0.0, 100.0, 100.0]));
    assert_eq!(shown(Fit::Fill).0, [0.0, 0.0, 100.0, 100.0]);
}

#[test]
fn a_large_image_is_pieces_that_meet_and_a_broken_one_is_nothing() {
    let mut h = Harness::new();
    h.mount(|| column().align_items(Align::Start).child(image(file(1200, 700)).width(600.0).height(350.0)));
    h.settle();
    // 600 by 350 pixels: two pieces across, one down, each on whole pixels, edge to edge.
    let painted = pieces(&h);
    assert_eq!(painted.len(), 2);
    assert_eq!((painted[0].bounds, painted[1].bounds), ([0.0, 0.0, 512.0, 350.0], [512.0, 0.0, 88.0, 350.0]));

    // What is far from the window is not uploaded: of a picture of six pieces in a page that
    // scrolls, the three in the band that is painted, the window and its height below it.
    let mut h = Harness::new();
    h.mount(|| scroll(signal(0.0), column().align_items(Align::Start).child(image(file(400, 3000)))));
    h.settle();
    assert_eq!(h.tree.fonts().borrow().color_glyphs.stats().tiles, 3);

    // No image, too large a claim, no file: the reason, and an empty box.
    for (source, wrong) in [
        (ImageSource::from(b"GIF89a, they say".to_vec()), "decoded"),
        (ImageSource::from(FlatImages::file(20_000, 20_000, 0)), "too large"),
        (ImageSource::from(".tmp/no-such-image.png"), "read"),
    ] {
        let mut h = Harness::new();
        let why = Rc::new(RefCell::new(String::new()));
        let told = why.clone();
        h.mount(move || {
            let failed = move |r: Result<Size, ImageError>| *told.borrow_mut() = r.unwrap_err().to_string();
            column().align_items(Align::Start).child(image(source).on_load(failed))
        });
        h.settle();
        assert!(why.borrow().contains(wrong), "{} lacks {wrong}", why.borrow());
        assert!(pieces(&h).is_empty() && h.tree.rect(first(&h)).size() == Size::ZERO);
    }
}

#[test]
fn a_view_that_comes_back_shows_in_its_first_frame() {
    let mut h = Harness::new();
    let there = signal(true);
    let source = file(64, 64);
    h.mount(move || {
        column()
            .align_items(Align::Start)
            .child(show(move || there.get(), move || image(source)))
            .child(image(source))
    });
    h.settle();
    // Two views of one file at one size: one picture, and its tile twice.
    assert_eq!((pieces(&h).len(), h.images.held()), (2, (1, 0)));
    assert_eq!(pieces(&h)[0].uv, pieces(&h)[1].uv);

    there.set(false);
    h.settle();
    assert_eq!(pieces(&h).len(), 1);
    // Back: laid out and painted in the frame it is mounted in, with no work on the pool.
    there.set(true);
    h.frame();
    assert_eq!(pieces(&h).len(), 2);
    assert_eq!(pieces(&h)[0].bounds, [0.0, 0.0, 64.0, 64.0]);
}

#[test]
fn an_image_that_moves_shows_its_frames_in_turn() {
    // Red, then blue, each for 100 ms, over and over; nothing of it is kept in the cache.
    let mut h = Harness::new();
    let file: &'static [u8] = b"flat 4 4 ff0000ff 0000ffff";
    h.mount(move || column().align_items(Align::Start).child(image(file)));
    h.settle();
    // The frame shown, by its picture: the first is the red one.
    let shown = |h: &mut Harness| {
        let id = first(h);
        let picture = h.tree.widget_mut::<Picture>(id).unwrap();
        Rc::as_ptr(picture.image().unwrap())
    };
    let first_frame = shown(&mut h);
    let red = |h: &mut Harness| shown(h) == first_frame;
    assert!(red(&mut h));
    h.advance(Duration::from_millis(100));
    h.settle();
    assert!(!red(&mut h));
    h.advance(Duration::from_millis(100));
    h.settle();
    assert!(red(&mut h));
    assert_eq!(h.images.held().0, 0);
}

#[test]
fn pixels_let_go_of_are_decoded_again_when_the_atlas_loses_them() {
    let mut h = Harness::new();
    h.mount(|| column().align_items(Align::Start).child(image(file(64, 64))));
    h.settle();
    let picture = |h: &mut Harness| h.tree.widget_mut::<Picture>(first(h)).unwrap().image().unwrap().clone();
    let before = picture(&mut h);
    assert!(!before.has_pixels() && pieces(&h).len() == 1);
    // The device is lost: every tile is forgotten, and everything painted again.
    h.fonts.borrow_mut().clear_tiles();
    h.tree.repaint_all();
    h.tree.scene_mut().invalidate_all();
    h.frame();
    h.settle();
    let after = picture(&mut h);
    assert!(!Rc::ptr_eq(&before, &after) && pieces(&h).len() == 1);
    assert!(!after.has_pixels());
}
