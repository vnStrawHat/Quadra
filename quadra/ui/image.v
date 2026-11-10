// Vector (svg, icon) and raster (img) image elements.
module ui

import hash.fnv1a
import math
import os
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
	// paint memo: the source hash and the image cache key for a device size and color. An
	// arena element keeps them, and parsed, while it shows the same source (arena.v).
	hash   u64
	key    string
	kw     int
	kh     int
	kcolor prim.Color
}

// svg draws inline SVG markup, rasterized at the exact device size and cached.
pub fn svg(src string) &SvgEl {
	return new_svg(src, 24)
}

// icon is a 16 px svg tinted with the current text color.
pub fn icon(src string) &SvgEl {
	return new_svg(src, 16)
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
	if s.hash == 0 {
		s.hash = fnv1a.sum64_string(s.src)
	}
	src_hash := s.hash
	if s.parsed == none {
		mut app := cx.app
		mut doc := &vector.Svg(unsafe { nil })
		if p := app.images.sources[src_hash] {
			doc = prim.held[vector.Svg](p)
		} else {
			parsed := vector.parse_svg(s.src) or { vector.Svg{} }
			doc = &vector.Svg{
				...parsed
			}
			app.images.sources[src_hash] = prim.hold(doc)
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
	if s.key == '' || s.kw != dw || s.kh != dh || (!mono && s.kcolor != color) {
		s.key = 'svg:${src_hash}:${dw}x${dh}' + if mono { '' } else { ':${color.hex()}' }
		s.kw, s.kh, s.kcolor = dw, dh, color
	}
	key := s.key
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

pub fn (mut i ImgEl) request_layout(mut cx core.LayoutCx) layout.NodeId {
	mut st := i.style
	mut app := cx.app
	e := app.image_async_with('img:${i.path}', i.path, decode_file)
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
