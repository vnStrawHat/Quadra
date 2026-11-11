//! Where tiles go in an atlas: shelves by height in the slices of a texture array, and the
//! queue of pixels a renderer uploads.

/// What an atlas holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtlasKind {
    /// 8-bit coverage: glyphs and monochrome icons.
    Coverage,
    /// RGBA: colour glyphs, small images, rasterized vectors.
    Color,
}

impl AtlasKind {
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            AtlasKind::Coverage => 1,
            AtlasKind::Color => 4,
        }
    }
}

/// A tile's place: a rectangle of pixels in one slice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TileRect {
    pub slice: u32,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Pixels to copy into the atlas texture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Upload {
    pub rect: TileRect,
    pub pixels: Vec<u8>,
}

/// Tiles of about the same height share a shelf: heights are rounded up to this.
const HEIGHT_STEP: u32 = 4;
/// Kept free around every tile, so sampling one never reads its neighbour.
const GAP: u32 = 1;

struct Shelf {
    y: u32,
    height: u32,
    /// Where the next tile goes when no freed span fits.
    x: u32,
    live: u32,
    /// Spans (x, width) given back by tiles that were dropped.
    free: Vec<(u32, u32)>,
}

#[derive(Default)]
struct Slice {
    shelves: Vec<Shelf>,
    next_y: u32,
}

/// The part of an atlas that does not depend on what its tiles are: packing, and what a
/// renderer needs to keep the texture in step.
pub struct AtlasCore {
    kind: AtlasKind,
    size: u32,
    slices: Vec<Slice>,
    uploads: Vec<Upload>,
    epoch: u64,
}

impl AtlasCore {
    pub(crate) fn new(kind: AtlasKind, size: u32) -> AtlasCore {
        debug_assert!(size.is_multiple_of(HEIGHT_STEP), "shelves are a multiple of {HEIGHT_STEP} high");
        AtlasCore { kind, size, slices: Vec::new(), uploads: Vec::new(), epoch: 1 }
    }

    pub fn kind(&self) -> AtlasKind {
        self.kind
    }

    /// The width and height of every slice, in pixels.
    pub fn slice_size(&self) -> u32 {
        self.size
    }

    /// How many slices the texture array must have.
    pub fn slices(&self) -> u32 {
        self.slices.len() as u32
    }

    /// Changes when everything was forgotten: a renderer starts its texture again.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// The pixels queued since the last call: a renderer copies them into its texture.
    pub fn take_uploads(&mut self) -> Vec<Upload> {
        std::mem::take(&mut self.uploads)
    }

    pub(crate) fn bytes_with(&self, slices: u32) -> usize {
        slices as usize * (self.size as usize).pow(2) * self.kind.bytes_per_pixel()
    }

    pub(crate) fn fits(&self, w: u32, h: u32) -> bool {
        w + GAP <= self.size && h + GAP <= self.size
    }

    pub(crate) fn queue(&mut self, rect: TileRect, pixels: &[u8]) {
        debug_assert_eq!(pixels.len(), (rect.w * rect.h) as usize * self.kind.bytes_per_pixel());
        self.uploads.push(Upload { rect, pixels: pixels.to_vec() });
    }

    pub(crate) fn add_slice(&mut self) {
        self.slices.push(Slice::default());
    }

    pub(crate) fn reset(&mut self) {
        self.slices.clear();
        self.uploads.clear();
        self.epoch += 1;
    }

    /// A place for a tile in the slices there are, or `None`.
    pub(crate) fn alloc(&mut self, w: u32, h: u32) -> Option<TileRect> {
        let (need_w, class) = (w + GAP, (h + GAP).next_multiple_of(HEIGHT_STEP));
        let size = self.size;
        for (index, slice) in self.slices.iter_mut().enumerate() {
            let place = |x, y| Some(TileRect { slice: index as u32, x, y, w, h });
            // A shelf of this height: a span a dropped tile gave back, or the end of the shelf.
            for shelf in slice.shelves.iter_mut().filter(|s| s.height == class) {
                if let Some(at) = shelf.free.iter().position(|&(_, span)| span >= need_w) {
                    let (x, span) = shelf.free[at];
                    if span > need_w {
                        shelf.free[at] = (x + need_w, span - need_w);
                    } else {
                        shelf.free.swap_remove(at);
                    }
                    shelf.live += 1;
                    return place(x, shelf.y);
                }
                if shelf.x + need_w <= size {
                    let x = shelf.x;
                    shelf.x += need_w;
                    shelf.live += 1;
                    return place(x, shelf.y);
                }
            }
            // An empty shelf of another height can be made one of this height, if it is the
            // last one or at least as high.
            let last = slice.shelves.len().saturating_sub(1);
            for (i, shelf) in slice.shelves.iter_mut().enumerate() {
                let room = if i == last { size - shelf.y } else { shelf.height };
                if shelf.live == 0 && class <= room {
                    if i == last {
                        (shelf.height, slice.next_y) = (class, shelf.y + class);
                    }
                    (shelf.x, shelf.live) = (need_w, 1);
                    shelf.free.clear();
                    return place(0, shelf.y);
                }
            }
            if slice.next_y + class <= size {
                slice.shelves.push(Shelf {
                    y: slice.next_y,
                    height: class,
                    x: need_w,
                    live: 1,
                    free: Vec::new(),
                });
                let y = slice.next_y;
                slice.next_y += class;
                return place(0, y);
            }
        }
        None
    }

    /// Gives a tile's place back.
    pub(crate) fn free(&mut self, rect: TileRect) {
        if rect.w == 0 || rect.h == 0 {
            return;
        }
        let Some(slice) = self.slices.get_mut(rect.slice as usize) else { return };
        let Some(shelf) = slice.shelves.iter_mut().find(|s| s.y == rect.y) else { return };
        shelf.live = shelf.live.saturating_sub(1);
        if shelf.live == 0 {
            // Empty: all of it is free again, in one piece.
            shelf.x = 0;
            shelf.free.clear();
        } else {
            shelf.free.push((rect.x, rect.w + GAP));
        }
    }
}
