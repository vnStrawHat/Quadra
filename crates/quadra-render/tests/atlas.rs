//! Atlases: packing, counted tiles, the budget (specs/07, "Atlases").

use quadra_render::{Atlas, AtlasKind, NullRenderer, Renderer};

/// A coverage atlas of 64 px slices whose budget is two slices.
fn atlas() -> Atlas<u32, &'static str> {
    Atlas::new(AtlasKind::Coverage, 64, 2 * 64 * 64)
}

fn pixels(w: u32, h: u32) -> Vec<u8> {
    vec![255; (w * h) as usize]
}

#[test]
fn tiles_are_packed_in_shelves_with_a_gap_and_queued_for_upload() {
    let mut a = atlas();
    let first = a.insert(1, 10, 10, &pixels(10, 10), "a").unwrap();
    let second = a.insert(2, 20, 9, &pixels(20, 9), "b").unwrap(); // the same shelf: 12 px high
    let tall = a.insert(3, 10, 30, &pixels(10, 30), "c").unwrap(); // another shelf
    assert_eq!((first.rect.x, first.rect.y, second.rect.x, second.rect.y), (0, 0, 11, 0));
    assert_eq!((tall.rect.x, tall.rect.y, tall.rect.slice), (0, 12, 0));
    assert_eq!(first.uv, [0.0, 0.0, 10.0 / 64.0, 10.0 / 64.0]);
    assert_eq!((second.meta, a.stats().slices), ("b", 1));

    let mut renderer = NullRenderer::new();
    renderer.sync_atlas(a.core_mut()).unwrap();
    assert_eq!(renderer.tiles_uploaded(), (3, 100 + 180 + 300));
    renderer.sync_atlas(a.core_mut()).unwrap(); // nothing new
    assert_eq!(renderer.tiles_uploaded().0, 3);
}

#[test]
fn a_key_has_one_tile_and_a_tile_of_no_size_takes_no_room() {
    let mut a = atlas();
    let first = a.insert(1, 8, 8, &pixels(8, 8), "a").unwrap();
    let again = a.insert(1, 8, 8, &pixels(8, 8), "other").unwrap();
    assert!(std::rc::Rc::ptr_eq(&first, &again) && again.meta == "a");
    let space = a.insert(2, 0, 0, &[], "space").unwrap();
    assert_eq!((space.rect.w, a.stats().tiles, a.core_mut().take_uploads().len()), (0, 2, 1));
    assert!(a.insert(3, 64, 10, &pixels(64, 10), "wide").is_none()); // no room for its gap
    assert!(a.get(&9).is_none());
}

#[test]
fn a_tile_nobody_holds_goes_first_and_a_tile_in_use_never_goes() {
    let mut a = atlas();
    // 28 px tiles: two a shelf, two shelves a slice, so eight tiles fill the budget.
    let held: Vec<_> = (0..4).map(|k| a.insert(k, 28, 28, &pixels(28, 28), "held").unwrap()).collect();
    for k in 4..8 {
        a.insert(k, 28, 28, &pixels(28, 28), "idle"); // the handle is dropped at once
    }
    assert_eq!((a.stats().slices, a.stats().in_use, a.stats().tiles), (2, 4, 8));
    a.get(&4); // used again: not the first to go

    let new = a.insert(100, 28, 28, &pixels(28, 28), "new").unwrap();
    let stats = a.stats();
    assert_eq!((stats.slices, stats.over_budget, stats.tiles), (2, false, 8)); // one idle tile made room
    assert!(a.get(&5).is_none() && a.get(&4).is_some());
    assert!(held.iter().all(|tile| a.get(&held_key(&held, tile)).is_some()));
    assert_eq!(new.rect.slice, 1); // in the place the dropped tile gave back
}

fn held_key<T>(all: &[std::rc::Rc<T>], tile: &std::rc::Rc<T>) -> u32 {
    all.iter().position(|t| std::rc::Rc::ptr_eq(t, tile)).unwrap() as u32
}

#[test]
fn the_atlas_grows_past_its_budget_only_when_everything_is_in_use() {
    let mut a = atlas();
    let held: Vec<_> = (0..8).map(|k| a.insert(k, 28, 28, &pixels(28, 28), "held").unwrap()).collect();
    assert_eq!((a.stats().slices, a.stats().over_budget), (2, false));
    let ninth = a.insert(8, 28, 28, &pixels(28, 28), "ninth").unwrap();
    assert_eq!((a.stats().slices, a.stats().over_budget, ninth.rect.slice), (3, true, 2));
    drop(held);
    assert_eq!(a.stats().in_use, 1);
}

#[test]
fn clearing_starts_a_new_epoch() {
    let mut a = atlas();
    let old = a.insert(1, 8, 8, &pixels(8, 8), "a").unwrap();
    a.clear(); // the device was lost
    assert_eq!((a.stats().tiles, a.stats().slices), (0, 0));
    let new = a.insert(1, 8, 8, &pixels(8, 8), "a").unwrap();
    assert!(new.epoch > old.epoch && new.epoch == a.epoch());
    assert_eq!(a.core_mut().take_uploads().len(), 1); // what was queued before is gone
}
