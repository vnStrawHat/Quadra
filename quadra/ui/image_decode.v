// Raster image decoding (PNG, JPEG, BMP, GIF) for img, on the worker thread image_async_with
// runs it on. Image files are untrusted input: the size is checked from the header before
// any pixel is decoded (specs/08, image caps).
module ui

import os
import stbi
import quadra.core

// A decoded image costs 4 bytes a pixel twice over (the decoder's buffer and the premultiplied
// copy): the pixel cap keeps one decode at 128 MiB, whatever a few KB of header claim. Larger
// images fail like a corrupt file. (The atlas shows up to 2047 px a side today, specs/03.)
pub const max_image_side = 16384
pub const max_image_pixels = 16 << 20
pub const max_image_file_bytes = 256 << 20

// max_pixels_per_byte bounds what a header may claim for the bytes that follow it: deflate
// expands a byte to at most 1032 (8 pixels each at one bit), LZW and JPEG less. A header
// claiming more is a lie the decoder would allocate for before finding no data.
pub const max_pixels_per_byte = 16384

fn C.stbi_info_from_memory(buffer &u8, len int, x &int, y &int, comp &int) int

fn decode_file(path string) !core.Bitmap {
	size := os.file_size(path)
	if size > u64(max_image_file_bytes) {
		return error('image file too large (${size} bytes)')
	}
	return decode_image(os.read_bytes(path)!)
}

// image_size reads the dimensions from the header without decoding.
pub fn image_size(bytes []u8) !(int, int) {
	if bytes.len == 0 || bytes.len > max_image_file_bytes {
		return error('not an image')
	}
	mut w, mut h, mut comp := 0, 0, 0
	if C.stbi_info_from_memory(bytes.data, bytes.len, &w, &h, &comp) == 0 {
		return error('not an image')
	}
	return w, h
}

const png_signature = [u8(0x89), 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]

// png_chunks_fit fails a PNG with a chunk longer than the bytes left: the decoder sizes its
// buffer for the image data from the length a chunk claims, before reading it (up to 1 GiB,
// doubled, from a 94-byte file).
fn png_chunks_fit(b []u8) ! {
	if b.len < 8 || b[..8] != png_signature {
		return
	}
	mut pos := 8
	for pos + 8 <= b.len {
		n := i64(u32(b[pos]) << 24 | u32(b[pos + 1]) << 16 | u32(b[pos + 2]) << 8 | u32(b[pos + 3]))
		if n > i64(b.len - pos - 12) {
			return error('PNG chunk longer than the file')
		}
		if b[pos + 4..pos + 8] == 'IEND'.bytes() {
			return
		}
		pos += 12 + int(n)
	}
}

// decode_image decodes an encoded image into premultiplied RGBA. Images over the caps, or
// whose header does not parse, fail before any pixel is decoded.
pub fn decode_image(bytes []u8) !core.Bitmap {
	w, h := image_size(bytes)!
	if w < 1 || h < 1 || w > max_image_side || h > max_image_side
		|| i64(w) * i64(h) > i64(max_image_pixels) {
		return error('image too large (${w}x${h})')
	}
	if i64(w) * i64(h) > i64(bytes.len) * max_pixels_per_byte {
		return error('image data too short for ${w}x${h}')
	}
	png_chunks_fit(bytes)!
	im := stbi.load_from_memory(bytes.data, bytes.len, desired_channels: 4)!
	defer {
		im.free()
	}
	if im.width != w || im.height != h { // the header said one size, the decoder another
		return error('inconsistent image size')
	}
	n := im.width * im.height * 4
	mut px := []u8{len: n}
	src := unsafe { &u8(im.data) }
	for i := 0; i < n; i += 4 {
		a := unsafe { src[i + 3] }
		for k in 0 .. 3 {
			px[i + k] = u8((u32(unsafe { src[i + k] }) * a + 127) / 255)
		}
		px[i + 3] = a
	}
	return core.Bitmap{im.width, im.height, px, false}
}
