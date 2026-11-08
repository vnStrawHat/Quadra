// PaintCx: the only place logical px become device px.
module core

import math
import quadra.prim
import quadra.gpu
import quadra.text

pub struct PaintCx {
pub mut:
	app        &App             = unsafe { nil }
	win        &Window          = unsafe { nil }
	scene      &gpu.Scene       = unsafe { nil }
	text       &text.TextSystem = unsafe { nil }
	scale      f32              = 1
	text_style text.TextStyle
	opacity    f32 = 1
	xf         Xform // push_transform
mut:
	masks      []gpu.ContentMask = [gpu.ContentMask{}]
	repatching bool // painting one element again for a patch: record nothing
}

pub fn (cx &PaintCx) is_hovered(hitbox int) bool {
	return hitbox >= 0 && hitbox < cx.win.hitboxes.len && cx.win.hovered[cx.win.hitboxes[hitbox].id]
}

pub fn (cx &PaintCx) is_active(hitbox int) bool {
	return hitbox >= 0 && hitbox < cx.win.hitboxes.len && cx.win.pressed[cx.win.hitboxes[hitbox].id]
}

pub fn (cx &PaintCx) is_focused(focus_id u64) bool {
	return focus_id != 0 && cx.win.focused == focus_id
}

// focus_visible reports whether focus came from the keyboard (show the focus ring).
pub fn (cx &PaintCx) focus_visible() bool {
	return cx.win.focus_visible
}

// Xform is a paint transform: logical point p paints at r((p * k + t) * scale), r the
// rotation of the subtree (device px; scene transform id, 0 when there is none).
pub struct Xform {
pub:
	k  f32 = 1
	t  prim.Point
	r  gpu.Rigid
	id u32
}

// push_transform rotates by deg degrees and scales by k about origin, then moves by d
// (logical px), whatever is painted until restore_transform(the returned value). Paint
// only: hit testing keeps the layout boxes. Text is rasterized again at the scaled size, in
// quarter px steps; a rotation turns the primitives on the GPU (specs/03, "Rotation").
pub fn (mut cx PaintCx) push_transform(origin prim.Point, d prim.Point, k f32, deg f32) Xform {
	saved := cx.xf
	tx, ty := origin.x - origin.x * k + d.x, origin.y - origin.y * k + d.y
	mut r, mut id := saved.r, saved.id
	if deg != 0 {
		// The pivot, origin moved by d, in the device space of the parent transform.
		px := ((origin.x + d.x) * saved.k + saved.t.x) * cx.scale
		py := ((origin.y + d.y) * saved.k + saved.t.y) * cx.scale
		r = saved.r.then(gpu.rotation_about(deg, px, py))
		id = cx.scene.push_xform(r)
	}
	cx.xf =
		Xform{saved.k * k, prim.Point{tx * saved.k + saved.t.x, ty * saved.k + saved.t.y}, r, id}
	return saved
}

pub fn (mut cx PaintCx) restore_transform(x Xform) {
	cx.xf = x
	cx.scene.set_xform(x.id)
}

// dev maps a logical rect to device px.
fn (cx &PaintCx) dev(r prim.Rect) prim.Rect {
	k, t, s := cx.xf.k, cx.xf.t, cx.scale
	return prim.rect((r.x * k + t.x) * s, (r.y * k + t.y) * s, r.width * k * s, r.height * k * s)
}

// clip_rect is the current clip in logical px (before the transform; around it when the
// clip was pushed under another rotation).
pub fn (cx &PaintCx) clip_rect() prim.Rect {
	m := cx.masks.last()
	b := if m.xform == cx.xf.id {
		m.bounds
	} else {
		cx.xf.r.inverse().bbox(cx.scene.xforms[m.xform].bbox(m.bounds))
	}
	k, t, s := cx.xf.k, cx.xf.t, cx.scale
	return prim.rect((b.x / s - t.x) / k, (b.y / s - t.y) / k, b.width / s / k, b.height / s / k)
}

fn (cx &PaintCx) mask() gpu.ContentMask {
	return cx.masks.last()
}

// push_clip clips everything painted until pop_clip to a (rounded) rect in logical px.
pub fn (mut cx PaintCx) push_clip(r prim.Rect, radius f32) {
	dev := cx.dev(r)
	parent := cx.masks.last()
	// A clip in another space than its parent's keeps its own rect (specs/03, "Rotation").
	same := parent.xform == cx.xf.id
	cx.masks << gpu.ContentMask{
		bounds: if same { parent.bounds.intersect(dev) } else { dev }
		radius: if radius > 0 || !same { radius * cx.xf.k * cx.scale } else { parent.radius }
		xform:  cx.xf.id
	}
}

pub fn (mut cx PaintCx) pop_clip() {
	cx.masks.pop()
}

fn (cx &PaintCx) fade(c prim.Color) prim.Color {
	return if cx.opacity >= 1 { c } else { c.alpha(c.a * cx.opacity) }
}

// quad_kept emits a quad even when it is invisible, so a hover-sensitive element draws the
// same number of primitives whatever its state (a ghost button gets a background on hover).
pub fn (mut cx PaintCx) quad_kept(q gpu.Quad) {
	if q.background.is_transparent() && q.border_widths == prim.Edges{} {
		cx.quad(gpu.Quad{
			...q
			background: prim.Color{0, 0, 0, 0.0001}
		})
		return
	}
	cx.quad(q)
}

pub fn (mut cx PaintCx) quad(q gpu.Quad) {
	s := cx.scale * cx.xf.k
	cx.scene.quad(gpu.Quad{
		bounds:         cx.dev(q.bounds)
		background:     cx.fade(q.background)
		gradient_to:    if g := q.gradient_to { ?prim.Color(cx.fade(g)) } else { none }
		gradient_angle: q.gradient_angle
		border_widths:  q.border_widths.scale(s)
		border_color:   cx.fade(q.border_color)
		radii:          q.radii.scale(s)
	}, cx.mask())
}

pub fn (mut cx PaintCx) shadow(sh gpu.Shadow) {
	s := cx.scale * cx.xf.k
	cx.scene.shadow(gpu.Shadow{
		bounds: cx.dev(sh.bounds)
		radii:  sh.radii.scale(s)
		blur:   sh.blur * s
		color:  cx.fade(sh.color)
	}, cx.mask())
}

pub fn (mut cx PaintCx) line(l gpu.LinePrim) {
	cx.scene.line(gpu.LinePrim{
		bounds:    cx.dev(l.bounds)
		color:     cx.fade(l.color)
		thickness: l.thickness * cx.scale * cx.xf.k
		wavy:      l.wavy
		column:    l.column
		rise:      l.rise * cx.scale * cx.xf.k
	}, cx.mask())
}

// stroke draws a polyline (logical px) of width w on the GPU (gpu.Scene.stroke).
pub fn (mut cx PaintCx) stroke(points []prim.Point, w f32, color prim.Color, closed bool) {
	k, t, s := cx.xf.k, cx.xf.t, cx.scale
	cx.scene.stroke(points.map(prim.pt((it.x * k + t.x) * s, (it.y * k + t.y) * s)), w * k * s,
		cx.fade(color), closed, cx.mask())
}

// glyphs paints a laid-out line with its baseline at (x, baseline), logical px.
pub fn (mut cx PaintCx) glyphs(l &text.LineLayout, x f32, baseline f32, color prim.Color) {
	cx.glyphs_range(l, x, baseline, color, 0, l.text.len)
}

// glyphs_range paints the glyphs whose cluster lies in the byte range [from, to) of the
// line text, so one layout can be painted in several colors without re-shaping. Glyphs of
// a ligature that spans a boundary take the color of its first character.
pub fn (mut cx PaintCx) glyphs_range(l &text.LineLayout, x f32, baseline f32, color prim.Color, from int, to int) {
	k, t := cx.xf.k, cx.xf.t
	s := cx.scale * k
	dev_size := if k == 1 { l.size * s } else { f32(math.round(l.size * s * 4)) / 4 }
	base_y := f32(math.round((baseline * k + t.y) * cx.scale))
	tx := t.x * cx.scale
	c := cx.fade(color)
	// Clusters and x of a cut line are measured from its base and origin (text.LineLayout).
	lo_b, hi_b := from + l.base, to + l.base
	x0 := x - l.origin
	// Glyphs are in cluster order: find the first one at or after from.
	mut lo, mut hi := 0, l.glyphs.len
	for lo < hi {
		mid := (lo + hi) / 2
		if l.glyphs[mid].cluster < lo_b {
			lo = mid + 1
		} else {
			hi = mid
		}
	}
	for gi in lo .. l.glyphs.len {
		g := l.glyphs[gi]
		if g.cluster >= hi_b {
			break
		}
		gx := (x0 + g.x) * s + tx
		fx := f32(math.floor(gx))
		phase := int((gx - fx) * text.subpixel_steps)
		tile := cx.text.glyph_tile(g.face, g.id, dev_size, phase)
		if tile.empty {
			continue
		}
		bounds := prim.rect(fx + tile.left, base_y - f32(math.round(g.y * s)) + tile.top,
			tile.tile.w, tile.tile.h)
		sprite := gpu.Sprite{
			bounds: bounds
			tile:   tile.tile
			color:  c
			glyph:  !tile.color
		}
		if tile.color || tile.subpixel {
			cx.scene.poly(sprite, cx.mask())
		} else {
			cx.scene.mono(sprite, cx.mask())
		}
	}
}

// sprite draws a cached bitmap (mono masks are tinted with color) at logical bounds.
pub fn (mut cx PaintCx) sprite(bounds prim.Rect, e &ImageEntry, color prim.Color) {
	if e.status != .ready {
		return
	}
	dev := cx.dev(bounds)
	sp := gpu.Sprite{
		bounds: prim.rect(f32(math.round(dev.x)), f32(math.round(dev.y)), dev.width, dev.height)
		tile:   e.tile
		color:  cx.fade(color)
	}
	if e.mono {
		cx.scene.mono(sp, cx.mask())
	} else {
		cx.scene.poly(gpu.Sprite{
			...sp
			color: prim.Color{1, 1, 1, cx.opacity}
		}, cx.mask())
	}
}

// request_paint_after repaints this window after ms (paint-only animations such as
// spinners and tooltips' delays). Repeated requests coalesce.
pub fn (mut cx PaintCx) request_paint_after(ms int) {
	mut w := cx.win
	if w.paint_timer != 0 {
		return
	}
	mut app := cx.app
	w.paint_timer = app.plat.set_timer(ms, false)
	app.timers[w.paint_timer] = TimerEntry{
		repaint: w.id
	}
}

pub fn (cx &PaintCx) now_ms() f64 {
	app := cx.app
	return app.plat.now_ms()
}

struct Deferred {
mut:
	el       Element
	bounds   prim.Rect
	priority int
}

// defer_draw schedules an element to prepaint and paint above the main tree (overlays).
pub fn (mut w Window) defer_draw(el Element, bounds prim.Rect, priority int) {
	w.deferred << Deferred{el, bounds, priority}
}
