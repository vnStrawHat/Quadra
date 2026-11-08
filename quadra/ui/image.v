// Vector (svg, icon) and raster (img) image elements.
module ui

import hash.fnv1a
import math
import os
import stbi
import quadra.core
import quadra.prim
import quadra.layout
import quadra.vector

@[heap]
pub struct SvgEl {
pub mut:
	src   string
	w     f32
	h     f32
	color ?prim.Color // default: the inherited text color
	style layout.Style
mut:
	parsed  ?vector.Svg
	node    layout.NodeId
	inherit prim.Color
}

// svg draws inline SVG markup, rasterized at the exact device size and cached.
pub fn svg(src string) &SvgEl {
	return &SvgEl{
		src: src
		w:   24
		h:   24
	}
}

// icon is a 16 px svg tinted with the current text color.
pub fn icon(src string) &SvgEl {
	return &SvgEl{
		src: src
		w:   16
		h:   16
	}
}

pub fn (self_ &SvgEl) size(v f32) &SvgEl {
	mut s := unsafe { self_ }
	s.w, s.h = v, v
	return s
}

pub fn (self_ &SvgEl) color(c prim.Color) &SvgEl {
	mut s := unsafe { self_ }
	s.color = c
	return s
}

pub fn (mut s SvgEl) request_layout(mut cx core.LayoutCx) layout.NodeId {
	s.inherit = cx.text_style.color
	mut st := s.style
	st.width, st.height = layout.px(s.w), layout.px(s.h)
	st.shrink = 0
	s.node = cx.tree.node(st, [])
	return s.node
}

pub fn (mut s SvgEl) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {}

pub fn (mut s SvgEl) paint(bounds prim.Rect, mut cx core.PaintCx) {
	src_hash := fnv1a.sum64_string(s.src)
	if s.parsed == none {
		mut app := cx.app
		mut doc := &vector.Svg(unsafe { nil })
		if p := app.images.sources[src_hash] {
			doc = unsafe { &vector.Svg(p) }
		} else {
			parsed := vector.parse_svg(s.src) or { vector.Svg{} }
			doc = &vector.Svg{
				...parsed
			}
			app.images.sources[src_hash] = voidptr(doc)
		}
		val := *doc
		s.parsed = val
	}
	art := s.parsed or { return }
	dw := int(math.round(bounds.width * cx.scale))
	dh := int(math.round(bounds.height * cx.scale))
	if dw <= 0 || dh <= 0 {
		return
	}
	color := s.color or { s.inherit }
	mono := art.is_single_color()
	key := 'svg:${src_hash}:${dw}x${dh}' + if mono { '' } else { ':${color.hex()}' }
	mut app := cx.app
	entry := app.cached_image(key) or {
		px := if mono { art.render_mask(dw, dh) } else { art.render_rgba(dw, dh, color) }
		app.image_put(key, core.Bitmap{dw, dh, px, mono})
	}
	cx.sprite(bounds, entry, color)
}

pub enum Fit {
	contain
	cover
	fill
}

@[heap]
pub struct ImgEl {
pub mut:
	path   string
	w      f32
	h      f32
	fit    Fit
	radius f32
	style  layout.Style
mut:
	node layout.NodeId
}

// img shows an image file (PNG, JPEG, BMP, GIF), decoded on a worker thread.
pub fn img(path string) &ImgEl {
	return &ImgEl{
		path: path
	}
}

pub fn (self_ &ImgEl) size(w f32, h f32) &ImgEl {
	mut i := unsafe { self_ }
	i.w, i.h = w, h
	return i
}

pub fn (self_ &ImgEl) fit(f Fit) &ImgEl {
	mut i := unsafe { self_ }
	i.fit = f
	return i
}

pub fn (self_ &ImgEl) rounded(r f32) &ImgEl {
	mut i := unsafe { self_ }
	i.radius = r
	return i
}

fn decode_file(path string) !core.Bitmap {
	bytes := os.read_bytes(path)!
	im := stbi.load_from_memory(bytes.data, bytes.len, desired_channels: 4)!
	defer {
		im.free()
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

pub fn (mut i ImgEl) request_layout(mut cx core.LayoutCx) layout.NodeId {
	mut st := i.style
	mut app := cx.app
	path := i.path
	e := app.image_async('img:${i.path}', fn [path] () !core.Bitmap {
		return decode_file(path)
	})
	w := if i.w > 0 {
		i.w
	} else if e.status == .ready {
		f32(e.width)
	} else {
		0
	}
	h := if i.h > 0 {
		i.h
	} else if e.status == .ready {
		f32(e.height)
	} else {
		0
	}
	st.width, st.height = layout.px(w), layout.px(h)
	i.node = cx.tree.node(st, [])
	return i.node
}

pub fn (mut i ImgEl) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.register_semantics(core.SemanticNode{
		role:   .image
		label:  os.file_name(i.path)
		bounds: bounds
	})
}

pub fn (mut i ImgEl) paint(bounds prim.Rect, mut cx core.PaintCx) {
	mut app := cx.app
	e := app.images.entries['img:${i.path}'] or { return }
	if e.status != .ready || e.width == 0 || e.height == 0 {
		return
	}
	iw, ih := f32(e.width), f32(e.height)
	r := match i.fit {
		.fill {
			bounds
		}
		.contain, .cover {
			k := if i.fit == .contain {
				prim.min(bounds.width / iw, bounds.height / ih)
			} else {
				prim.max(bounds.width / iw, bounds.height / ih)
			}
			prim.rect(bounds.x + (bounds.width - iw * k) / 2, bounds.y +
				(bounds.height - ih * k) / 2, iw * k, ih * k)
		}
	}

	cx.push_clip(bounds, i.radius)
	cx.sprite(r, e, prim.white)
	cx.pop_clip()
}
