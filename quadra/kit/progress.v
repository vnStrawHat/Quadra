// Progress feedback: progress bar, spinner, skeleton. Animations are paint-only: they
// never re-render or relayout, they only repaint on a 16 ms tick while visible.
module kit

import math
import quadra.core
import quadra.prim
import quadra.gpu
import quadra.layout

@[heap]
pub struct Progress {
pub mut:
	value         f32 // 0..1
	indeterminate bool
	width         f32 = 200
	height        f32 = 6
mut:
	node layout.NodeId
}

pub fn progress(value f32) &Progress {
	return &Progress{
		value: value
	}
}

pub fn progress_indeterminate() &Progress {
	return &Progress{
		indeterminate: true
	}
}

pub fn (self_ &Progress) width(w f32) &Progress {
	mut p := unsafe { self_ }
	p.width = w
	return p
}

pub fn (mut p Progress) request_layout(mut cx core.LayoutCx) layout.NodeId {
	p.node = cx.tree.node(layout.Style{
		width:  layout.px(p.width)
		height: layout.px(p.height)
		shrink: 0
	}, [])
	return p.node
}

pub fn (mut p Progress) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.register_semantics(core.SemanticNode{
		role:   .progressbar
		value:  if p.indeterminate { '' } else { '${int(p.value * 100)}%' }
		bounds: bounds
	})
}

pub fn (mut p Progress) paint(bounds prim.Rect, mut cx core.PaintCx) {
	t := theme_of(cx.app)
	r := prim.corners(bounds.height / 2)
	cx.quad(gpu.Quad{
		bounds:     bounds
		background: t.p.chrome // the track is the chrome tone, the fill the accent (specs/15)
		radii:      r
	})
	cx.push_clip(bounds, bounds.height / 2)
	if p.indeterminate {
		phase := f32(math.fmod(cx.now_ms(), 1400) / 1400)
		w := bounds.width * 0.35
		x := bounds.x - w + (bounds.width + w) * phase
		cx.quad(gpu.Quad{
			bounds:     prim.rect(x, bounds.y, w, bounds.height)
			background: t.p.accent
			radii:      r
		})
		cx.request_paint_after(16)
	} else {
		cx.quad(gpu.Quad{
			bounds:     prim.rect(bounds.x, bounds.y, bounds.width * prim.clamp(p.value, 0, 1),
				bounds.height)
			background: t.p.accent
			radii:      r
		})
	}
	cx.pop_clip()
}

@[heap]
pub struct Spinner {
pub mut:
	diameter f32 = 16
	color    ?prim.Color
mut:
	node    layout.NodeId
	inherit prim.Color
}

pub fn spinner() &Spinner {
	return &Spinner{}
}

pub fn (self_ &Spinner) size(v f32) &Spinner {
	mut s := unsafe { self_ }
	s.diameter = v
	return s
}

pub fn (mut s Spinner) request_layout(mut cx core.LayoutCx) layout.NodeId {
	s.inherit = cx.text_style.color
	s.node = cx.tree.node(layout.Style{
		width:  layout.px(s.diameter)
		height: layout.px(s.diameter)
		shrink: 0
	}, [])
	return s.node
}

pub fn (mut s Spinner) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {}

pub fn (mut s Spinner) paint(bounds prim.Rect, mut cx core.PaintCx) {
	// A three-quarter arc stroked on the GPU at the exact angle of this frame: smooth at any
	// frame rate, and nothing goes through the atlas.
	start := math.fmod(cx.now_ms(), 900) / 900 * 2 * math.pi
	r := bounds.width / 2 - 1.5
	sweep := 1.5 * math.pi
	n := int(prim.clamp(f32(r * sweep) * cx.scale / 3, 8, 64)) // about 3 device px a segment
	mut pts := []prim.Point{cap: n + 1}
	for i in 0 .. n + 1 {
		a := start + sweep * f64(i) / f64(n)
		pts << prim.pt(bounds.x + bounds.width / 2 + r * f32(math.cos(a)), bounds.y +
			bounds.height / 2 + r * f32(math.sin(a)))
	}
	cx.stroke(pts, 2, s.color or { s.inherit }, false)
	cx.request_paint_after(16)
}

// Skeleton is a pulsing placeholder block for content still loading.
@[heap]
pub struct Skeleton {
pub mut:
	w      f32
	h      f32
	radius f32 = 6
mut:
	node layout.NodeId
}

pub fn skeleton(w f32, h f32) &Skeleton {
	return &Skeleton{
		w: w
		h: h
	}
}

pub fn (mut s Skeleton) request_layout(mut cx core.LayoutCx) layout.NodeId {
	s.node = cx.tree.node(layout.Style{
		width:  if s.w > 0 { layout.px(s.w) } else { layout.pct(100) }
		height: layout.px(s.h)
		shrink: 0
	}, [])
	return s.node
}

pub fn (mut s Skeleton) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {}

pub fn (mut s Skeleton) paint(bounds prim.Rect, mut cx core.PaintCx) {
	t := theme_of(cx.app)
	pulse := f32(0.55 + 0.45 * math.sin(cx.now_ms() / 1000 * math.pi * 2 * 0.7))
	cx.quad(gpu.Quad{
		bounds:     bounds
		background: t.p.raised_hover.lerp(t.p.press, pulse)
		radii:      prim.corners(s.radius)
	})
	cx.request_paint_after(33)
}
