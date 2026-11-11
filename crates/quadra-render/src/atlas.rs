//! Atlases (specs/07, "Atlases"): tiles packed in the slices of one texture array, counted by
//! who paints with them.
//!
//! A tile lives while something holds its handle. When the last handle drops the tile stays
//! usable, and is the first to go when room is needed. A tile in use is never evicted, so a
//! display list kept across frames never points at a rectangle that was given to another tile.

use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

use crate::hasher::FastBuild;
use crate::packing::{AtlasCore, AtlasKind, TileRect};

/// A rectangle of an atlas and what its owner knows about it (a glyph's offsets).
#[derive(Debug)]
pub struct Tile<M> {
    pub rect: TileRect,
    /// u0, v0, u1, v1 in its slice.
    pub uv: [f32; 4],
    pub meta: M,
    /// The atlas epoch it was made in: a tile of an earlier epoch is gone from the GPU.
    pub epoch: u64,
}

/// A handle to a tile: holding one keeps the tile in the atlas.
pub type TileRef<M> = Rc<Tile<M>>;

struct Entry<M> {
    tile: TileRef<M>,
    used: u64,
}

/// What an atlas holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AtlasStats {
    pub tiles: usize,
    /// Tiles something holds a handle to.
    pub in_use: usize,
    pub slices: u32,
    pub bytes: usize,
    /// It grew past its budget because every tile was in use.
    pub over_budget: bool,
}

pub struct Atlas<K, M> {
    core: AtlasCore,
    tiles: HashMap<K, Entry<M>, FastBuild>,
    clock: u64,
    budget: usize,
    over_budget: bool,
}

impl<K: Hash + Eq + Clone, M> Atlas<K, M> {
    /// An atlas of square slices of `slice_size` pixels that tries to stay within `budget`
    /// bytes.
    pub fn new(kind: AtlasKind, slice_size: u32, budget: usize) -> Atlas<K, M> {
        Atlas {
            core: AtlasCore::new(kind, slice_size),
            tiles: HashMap::default(),
            clock: 0,
            budget,
            over_budget: false,
        }
    }

    /// The part a renderer uploads from.
    pub fn core_mut(&mut self) -> &mut AtlasCore {
        &mut self.core
    }

    pub fn epoch(&self) -> u64 {
        self.core.epoch()
    }

    /// The tile of a key, if it is in the atlas. Getting it counts as using it.
    pub fn get(&mut self, key: &K) -> Option<TileRef<M>> {
        let entry = self.tiles.get_mut(key)?;
        self.clock += 1;
        entry.used = self.clock;
        Some(entry.tile.clone())
    }

    /// Adds a tile of `w` by `h` pixels (one byte a pixel for coverage, four for colour) and
    /// queues its upload. `None` when it is larger than a slice. A tile of no size takes no
    /// room.
    pub fn insert(&mut self, key: K, w: u32, h: u32, pixels: &[u8], meta: M) -> Option<TileRef<M>> {
        if let Some(tile) = self.get(&key) {
            return Some(tile); // already there: its place may be in use
        }
        let rect = if w == 0 || h == 0 { TileRect::default() } else { self.make_room(w, h)? };
        if w > 0 && h > 0 {
            self.core.queue(rect, pixels);
        }
        let size = self.core.slice_size() as f32;
        let (x, y) = (rect.x as f32, rect.y as f32);
        let uv = [x / size, y / size, (x + w as f32) / size, (y + h as f32) / size];
        let tile = Rc::new(Tile { rect, uv, meta, epoch: self.core.epoch() });
        self.clock += 1;
        self.tiles.insert(key, Entry { tile: tile.clone(), used: self.clock });
        Some(tile)
    }

    /// Finds room: in what is there, then in a new slice while the budget allows, then by
    /// dropping tiles nobody holds, and past the budget only when every tile is in use.
    fn make_room(&mut self, w: u32, h: u32) -> Option<TileRect> {
        if !self.core.fits(w, h) {
            return None;
        }
        loop {
            if let Some(rect) = self.core.alloc(w, h) {
                return Some(rect);
            }
            let next = self.core.bytes_with(self.core.slices() + 1);
            if next <= self.budget || !self.evict() {
                self.over_budget |= next > self.budget;
                self.core.add_slice();
            }
        }
    }

    /// Drops the least recently used quarter of the tiles nobody holds; false if there are
    /// none.
    fn evict(&mut self) -> bool {
        let mut idle: Vec<(u64, K)> = self
            .tiles
            .iter()
            .filter(|(_, e)| Rc::strong_count(&e.tile) == 1)
            .map(|(k, e)| (e.used, k.clone()))
            .collect();
        if idle.is_empty() {
            return false;
        }
        idle.sort_unstable_by_key(|(used, _)| *used);
        for (_, key) in idle.iter().take(idle.len().div_ceil(4)) {
            if let Some(entry) = self.tiles.remove(key) {
                self.core.free(entry.tile.rect);
            }
        }
        true
    }

    /// Forgets every tile: the GPU lost them (a new device). Handles still held are of an
    /// earlier epoch; their holders paint again and get new tiles.
    pub fn clear(&mut self) {
        self.tiles.clear();
        self.core.reset();
        self.over_budget = false;
    }

    pub fn stats(&self) -> AtlasStats {
        AtlasStats {
            tiles: self.tiles.len(),
            in_use: self.tiles.values().filter(|e| Rc::strong_count(&e.tile) > 1).count(),
            slices: self.core.slices(),
            bytes: self.core.bytes_with(self.core.slices()),
            over_budget: self.over_budget,
        }
    }
}
