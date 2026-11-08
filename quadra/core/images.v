// Image cache: bitmaps (decoded images, rasterized vectors) in the shared atlases.
// Synchronous for cheap vector work, asynchronous (worker thread) for decoding.
module core

import quadra.gpu

pub struct Bitmap {
pub:
	width  int
	height int
	pixels []u8 // RGBA premultiplied, or 8-bit coverage when mono
	mono   bool
}

pub enum ImageStatus {
	loading
	ready
	failed
}

@[heap]
pub struct ImageEntry {
pub mut:
	status ImageStatus
	tile   gpu.AtlasTile
	width  int
	height int
	mono   bool
	error  string
}

pub struct ImageCache {
pub mut:
	entries map[string]&ImageEntry
	pending int // decodes in flight
	// sources holds decoded sources by content hash (parsed SVG documents), so elements
	// rebuilt every frame do not parse their source again.
	sources map[u64]voidptr
}

fn (mut app App) store_bitmap(mut e ImageEntry, b Bitmap) {
	tile := if b.mono {
		app.text.mono.insert(b.width, b.height, b.pixels) or {
			e.status, e.error = .failed, err.msg()
			return
		}
	} else {
		app.text.poly.insert(b.width, b.height, b.pixels) or {
			e.status, e.error = .failed, err.msg()
			return
		}
	}
	e.tile, e.width, e.height, e.mono, e.status = tile, b.width, b.height, b.mono, .ready
}

// cached_image returns the entry for key if it exists. Paint code checks it, then renders the
// bitmap inline and stores it with image_put: no closure, since one made in paint has no
// owner and is never freed (specs/14).
pub fn (app &App) cached_image(key string) ?&ImageEntry {
	return app.images.entries[key] or { return none }
}

// image_put stores a bitmap produced on the calling thread under key.
pub fn (mut app App) image_put(key string, b Bitmap) &ImageEntry {
	mut e := &ImageEntry{}
	app.images.entries[key] = e
	app.store_bitmap(mut e, b)
	return e
}

// image_async returns the entry at once (loading at first) and decodes on a worker
// thread; when the bitmap arrives every window re-lays out (sizes may change).
pub fn (mut app App) image_async(key string, load fn () !Bitmap) &ImageEntry {
	if e := app.images.entries[key] {
		return e
	}
	mut e := &ImageEntry{}
	app.images.entries[key] = e
	app.images.pending++
	spawn fn (mut app App, mut e ImageEntry, load fn () !Bitmap) {
		mut failure := ''
		bmp := load() or {
			failure = err.msg()
			Bitmap{}
		}
		app.plat.post(fn [mut app, mut e, bmp, failure] () {
			app.images.pending--
			if failure == '' {
				app.store_bitmap(mut e, bmp)
			} else {
				e.status, e.error = .failed, failure
			}
			for mut w in app.windows {
				w.invalidate()
			}
		})
	}(mut app, mut e, load)
	return e
}
