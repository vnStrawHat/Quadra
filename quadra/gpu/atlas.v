// Texture atlases (pure V): shelf packing into fixed-size pages, with uploads queued
// for the device. ponytail: no eviction yet; pages are added until memory says stop.
module gpu

pub enum AtlasKind {
	mono // R8 coverage
	poly // RGBA8 premultiplied
}

pub struct AtlasTile {
pub:
	page int
	x    int
	y    int
	w    int
	h    int
	u0   f32
	v0   f32
	u1   f32
	v1   f32
}

pub struct Upload {
pub:
	kind   AtlasKind
	page   int
	x      int
	y      int
	w      int
	h      int
	pixels []u8
	// owned pixels live in the C heap and pass to the upload: the window frees them once the
	// device has copied them (free_owned); a device that keeps an upload copies them.
	owned bool
}

struct Shelf {
mut:
	y      int
	height int
	x      int
}

struct Page {
mut:
	shelves []Shelf
	next_y  int
}

pub struct Atlas {
pub:
	kind      AtlasKind
	page_size int = 2048
mut:
	pages   []Page
	uploads []Upload
pub mut:
	bytes_uploaded i64
}

// first_page_max caps the first page of each atlas: most apps never fill it, and a 2048 RGBA
// page costs 16 MiB of video memory and as much commit charge (the driver's backing), a
// 1024 one 4 MiB. Later pages, and tiles that do not fit it, take the full page size.
pub const first_page_max = 1024

// page_extent is the width and height of page `index` of an atlas with pages of page_size.
pub fn page_extent(page_size int, index int) int {
	return if index == 0 && page_size > first_page_max { first_page_max } else { page_size }
}

pub fn new_atlas(kind AtlasKind, page_size int) Atlas {
	return Atlas{
		kind:      kind
		page_size: page_size
	}
}

pub fn (a &Atlas) page_count() int {
	return a.pages.len
}

// insert packs a w x h bitmap and queues its upload. One px of padding avoids bleeding.
pub fn (mut a Atlas) insert(w int, h int, pixels []u8) !AtlasTile {
	return a.place(w, h, pixels, false)
}

// insert_owned is insert for pixels in the C heap that the upload takes over (Upload.owned).
// On an error nothing is queued and the caller still owns them.
pub fn (mut a Atlas) insert_owned(w int, h int, pixels []u8) !AtlasTile {
	return a.place(w, h, pixels, true)
}

fn (mut a Atlas) place(w int, h int, pixels []u8, owned bool) !AtlasTile {
	if w <= 0 || h <= 0 {
		return AtlasTile{}
	}
	if w + 1 > a.page_size || h + 1 > a.page_size {
		return error('atlas tile ${w}x${h} exceeds page size ${a.page_size}')
	}
	for p in 0 .. a.pages.len {
		if t := a.try_page(p, w, h) {
			return a.finish(t, pixels, owned)
		}
	}
	a.pages << Page{}
	if t := a.try_page(a.pages.len - 1, w, h) {
		return a.finish(t, pixels, owned)
	}
	a.pages << Page{} // too big for the small first page
	t := a.try_page(a.pages.len - 1, w, h) or { return error('atlas allocation failed') }
	return a.finish(t, pixels, owned)
}

fn (mut a Atlas) try_page(p int, w int, h int) ?AtlasTile {
	mut page := &a.pages[p]
	size := page_extent(a.page_size, p)
	need_w, need_h := w + 1, h + 1
	// Best shelf: tall enough, not wastefully tall, with room left.
	mut best := -1
	for i, s in page.shelves {
		if s.height >= need_h && s.height <= need_h + need_h / 2 + 2 && s.x + need_w <= size {
			if best < 0 || s.height < page.shelves[best].height {
				best = i
			}
		}
	}
	if best < 0 {
		if page.next_y + need_h > size || need_w > size {
			return none
		}
		page.shelves << Shelf{
			y:      page.next_y
			height: need_h
		}
		page.next_y += need_h
		best = page.shelves.len - 1
	}
	mut s := &page.shelves[best]
	x, y := s.x, s.y
	s.x += need_w
	inv := 1 / f32(size)
	return AtlasTile{p, x, y, w, h, f32(x) * inv, f32(y) * inv, f32(x + w) * inv, f32(y + h) * inv}
}

fn (mut a Atlas) finish(t AtlasTile, pixels []u8, owned bool) AtlasTile {
	a.uploads << Upload{a.kind, t.page, t.x, t.y, t.w, t.h, pixels, owned}
	a.bytes_uploaded += pixels.len
	return t
}

// rewrite replaces the pixels of a tile (same size), for images that reuse a dead tile;
// owned as for insert_owned.
pub fn (mut a Atlas) rewrite(t AtlasTile, pixels []u8, owned bool) AtlasTile {
	return a.finish(t, pixels, owned)
}

// take_uploads hands queued uploads to the device and clears the queue.
pub fn (mut a Atlas) take_uploads() []Upload {
	out := a.uploads
	a.uploads = []Upload{}
	return out
}
