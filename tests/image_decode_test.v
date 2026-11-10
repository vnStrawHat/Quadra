// Fuzz test of the raster image decoder entry (specs/11, fuzz tests; specs/08, image caps):
// tiny valid PNG, JPEG, BMP and GIF images and their mutations, headers claiming huge sizes.
import os
import stbi
import quadra.core
import quadra.ui
import quadra.testing.fuzz

// A 1x1 GIF (GIF89a, one color, one LZW-coded pixel).
const tiny_gif = [u8(0x47), 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00, 0x01, 0x00, 0x80, 0x00, 0x00,
	0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x21, 0xf9, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00, 0x2c, 0x00,
	0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x02, 0x02, 0x44, 0x01, 0x00, 0x3b]

fn corpus() ![][]u8 {
	dir := os.join_path(os.dir(@FILE), '..', '.tmp', 'fuzz-images')
	os.mkdir_all(dir)!
	mut px := []u8{len: 5 * 3 * 4}
	for i in 0 .. px.len {
		px[i] = u8(i * 37)
	}
	png := os.join_path(dir, 'a.png')
	bmp := os.join_path(dir, 'a.bmp')
	jpg := os.join_path(dir, 'a.jpg')
	stbi.stbi_write_png(png, 5, 3, 4, px.data, 5 * 4)!
	stbi.stbi_write_bmp(bmp, 5, 3, 4, px.data)!
	stbi.stbi_write_jpg(jpg, 5, 3, 4, px.data, 80)!
	return [os.read_bytes(png)!, os.read_bytes(bmp)!, os.read_bytes(jpg)!, tiny_gif]
}

fn test_corpus_decodes() {
	for b in corpus()! {
		bmp := ui.decode_image(b)!
		assert bmp.width > 0 && bmp.pixels.len == bmp.width * bmp.height * 4
	}
}

// put_be/put_le write a size field into a header.
fn put_be(mut b []u8, at int, v u32) {
	b[at], b[at + 1], b[at + 2], b[at + 3] = u8(v >> 24), u8(v >> 16), u8(v >> 8), u8(v)
}

fn put_le16(mut b []u8, at int, v u32) {
	b[at], b[at + 1] = u8(v), u8(v >> 8)
}

fn put_le32(mut b []u8, at int, v u32) {
	put_le16(mut b, at, v)
	put_le16(mut b, at + 2, v >> 16)
}

// Headers that claim more pixels than the caps are refused before decoding: no allocation
// in proportion to what a file says about itself.
fn test_huge_headers_are_refused() {
	c := corpus()!
	mut png := c[0].clone() // IHDR width and height at 16 and 20
	put_be(mut png, 16, 100000)
	put_be(mut png, 20, 100000)
	assert ui.decode_image(png) or { core.Bitmap{} }.width == 0
	mut bmp := c[1].clone() // BITMAPINFOHEADER width and height at 18 and 22
	put_le32(mut bmp, 18, 30000)
	put_le32(mut bmp, 22, 30000)
	assert ui.decode_image(bmp) or { core.Bitmap{} }.width == 0
	mut gif := tiny_gif.clone() // logical screen and image descriptor sizes
	put_le16(mut gif, 6, 65535)
	put_le16(mut gif, 8, 65535)
	put_le16(mut gif, 32, 65535)
	put_le16(mut gif, 34, 65535)
	assert ui.decode_image(gif) or { core.Bitmap{} }.width == 0
	// Sizes the decoder itself would accept: over the side cap, then over the pixel cap.
	put_be(mut png, 16, 20000)
	put_be(mut png, 20, 10000)
	w, h := ui.image_size(png)!
	assert w == 20000 && h == 10000
	assert ui.decode_image(png) or { core.Bitmap{} }.width == 0
	put_be(mut png, 16, 16000)
	put_be(mut png, 20, 8000)
	assert ui.decode_image(png) or {
		assert err.msg().contains('too large')
		core.Bitmap{}
	}.width == 0
	assert ui.max_image_side == 16384 && ui.max_image_pixels == 16 << 20
}

fn test_fuzz_image_decoder() {
	c := corpus()!
	mut r := fuzz.new_rng(0x1a6e)
	n := fuzz.iters(2000)
	mut w := fuzz.watch('image decode', n)
	mut ok := 0
	for i in 0 .. n {
		src := c[r.intn(c.len)]
		mut b := r.mutate(src, c)
		if r.chance(30) && b.len >= 28 { // a size field set to a boundary value
			put_be(mut b, 16 + r.intn(8), fuzz.boundary_u32[r.intn(fuzz.boundary_u32.len)])
		}
		w.start()
		if bmp := ui.decode_image(b) {
			assert bmp.width <= ui.max_image_side && bmp.height <= ui.max_image_side
			assert bmp.pixels.len == bmp.width * bmp.height * 4
			ok++
		}
		w.done(i, b.len, 2000)
	}
	w.finish(64)
	assert ok > 0 // some mutations still decode: the inputs reach the decoder
}

// Regression: a PNG chunk claiming 256 MiB made the decoder allocate that much (doubled) for
// a 94-byte file; chunk lengths are now checked against the file first.
fn test_png_chunk_longer_than_file_is_refused() {
	c := corpus()!
	mut png := c[0].clone()
	put_be(mut png, 33, 0x10000024) // the length of the chunk after IHDR
	before := gc_heap_usage().total_bytes
	assert ui.decode_image(png) or {
		assert err.msg().contains('chunk')
		core.Bitmap{}
	}.width == 0
	assert gc_heap_usage().total_bytes - before < 1 << 20 // bytes allocated by the call
}

// Regression: a 34-byte GIF claiming 8448x1273 made the decoder allocate its frame buffers
// (about 130 MiB) before finding no data. No format packs more than a few thousand pixels in
// a byte, so a header claiming more than max_pixels_per_byte is refused at once.
fn test_header_bigger_than_its_data_is_refused() {
	mut gif := tiny_gif.clone()
	put_le16(mut gif, 6, 8448)
	put_le16(mut gif, 8, 1273)
	put_le16(mut gif, 32, 8448)
	put_le16(mut gif, 34, 1273)
	before := gc_heap_usage().total_bytes
	assert ui.decode_image(gif) or {
		assert err.msg().contains('short')
		core.Bitmap{}
	}.width == 0
	assert gc_heap_usage().total_bytes - before < 1 << 20
}
