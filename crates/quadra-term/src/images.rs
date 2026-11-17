//! Images (specs/18, "Images (Sixel)"): the engine's decoded pixels, premultiplied once on the
//! reader thread, taken by the UI thread at its next refresh, and painted from pieces of the
//! colour atlas at the top-left cell of each placement that shows.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use oneterm_vt::{GraphicData, GraphicId, SnapshotState};
use quadra_platform::Decoded;
use quadra_prim::Rect;
use quadra_tree::{GlyphTile, Image, PaintCx, paint_image};

use crate::session::Session;
use crate::widget::TerminalWidget;

/// What the reader thread took from the engine since the UI thread last looked, behind the
/// session's lock.
#[derive(Default)]
pub(crate) struct Arrivals {
    new: Vec<(GraphicId, Decoded)>,
    released: Vec<GraphicId>,
}

impl Arrivals {
    /// An image the engine decoded: its pixels premultiplied, once.
    pub fn arrive(&mut self, data: Arc<GraphicData>) {
        let mut data = Arc::unwrap_or_clone(data);
        for pixel in data.rgba.chunks_exact_mut(4) {
            let alpha = u16::from(pixel[3]);
            for c in &mut pixel[..3] {
                *c = ((u16::from(*c) * alpha + 127) / 255) as u8;
            }
        }
        let pixels = Decoded { width: data.width, height: data.height, pixels: data.rgba };
        self.new.push((data.id, pixels));
    }

    /// An image the engine let go of (`GraphicReleased`): one not taken yet never reaches the
    /// UI thread, so a session without a view holds no more than the engine keeps alive.
    pub fn release(&mut self, id: GraphicId) {
        let before = self.new.len();
        self.new.retain(|it| it.0 != id);
        if self.new.len() == before {
            self.released.push(id);
        }
    }
}

/// The images a session shows, on the UI thread, and the atlas tiles its last paint holds.
/// An image keeps its pixels while it lives: the engine hands them out once, so a piece the
/// atlas loses is made again from them (the engine's limit of live pixels bounds them).
#[derive(Default)]
pub(crate) struct Images {
    shown: HashMap<GraphicId, Rc<Image>>,
    tiles: Vec<GlyphTile>,
}

impl Images {
    /// Takes what arrived and drops what was let go of: in a refresh, under the lock.
    pub fn take(&mut self, from: &mut Arrivals) {
        for (id, pixels) in from.new.drain(..) {
            self.shown.insert(id, Image::new(pixels));
        }
        for id in from.released.drain(..) {
            self.shown.remove(&id);
        }
    }
}

impl Session {
    /// The size in pixels of each image the session shows (those its view has taken).
    pub fn images(&self) -> Vec<(u32, u32)> {
        self.0.images.borrow().shown.values().map(|it| (it.width, it.height)).collect()
    }
}

/// Paints each placement that meets the viewport, under the text and clipped to the grid: at
/// its top-left cell's origin plus its offset, `display_size` device pixels.
// ponytail: the whole image is drawn (a Sixel's case); kitty's crop (`source_origin`,
// `source_size`) and layers over the text when kitty graphics are enabled.
pub(crate) fn paint(w: &TerminalWidget, cx: &mut PaintCx, state: &SnapshotState, (left, top): (f32, f32)) {
    let mut images = w.session.0.images.borrow_mut();
    let rows = state.rows();
    let Some(first) = rows.first().filter(|_| !state.placements().is_empty()) else {
        images.tiles.clear();
        return;
    };
    let (n, cols) = (rows.len() as u64, first.cells.len() as f32);
    let first = first.id.0;
    let grid = cx.device(Rect::new(left, top, cols * w.cell, n as f32 * w.line));
    let clip = cx.clip.then(grid, 0.0);
    let mut tiles = Vec::new();
    for p in state.placements() {
        // The other screen's rows are 2^63 away: they never meet the viewport.
        let (start, end) = (p.row.0, p.row.0.saturating_add(u64::from(p.rows)));
        if end <= first || start >= first.saturating_add(n) {
            continue;
        }
        let Some(image) = images.shown.get(&p.image).cloned() else { continue };
        let r = start.wrapping_sub(first) as i64 as f32;
        let x = ((left + f32::from(p.col) * w.cell) * cx.scale).round() + f32::from(p.pixel_offset.0);
        let y = ((top + r * w.line) * cx.scale).round() + f32::from(p.pixel_offset.1);
        let dest = Rect::new(x, y, p.display_size.0 as f32, p.display_size.1 as f32);
        paint_image(cx, &image, dest, &clip, &mut tiles);
    }
    images.tiles = tiles; // the old ones are let go after the new are held
}
