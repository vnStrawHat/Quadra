// Auto-hiding scroll thumbs (specs/15). A thumb shows while the pointer moves over its
// container, the wheel scrolls there, or the content moves by any other means (keys, a
// drag, the app); it stays 1 s after the last such activity, then fades out over 400 ms.
// A shown thumb can be dragged. One record per window says where and when the last
// activity was, so containers keep no state of their own.
module core

import quadra.prim
import quadra.gpu

// ThumbStyle is how scroll thumbs look (specs/15, Scrollbar): One's 6 px pill inset 4,
// at least 25 long, shown 1 s and fading over 400 ms. A zero color takes the window's
// scroll color.
pub struct ThumbStyle {
pub:
	width   f32 = 6
	wide    f32 = 6 // under the pointer or while dragged
	inset   f32 = 4 // from the side edge
	min_len f32 = 25
	radius  f32 = 3
	color   prim.Color
	hover   prim.Color // under the pointer or dragged; zero: color
	hold_ms f64 = 1000
	fade_ms f64 = 400
	always  bool // never hides
}

// ThumbSet moves a container to `offset` px along an axis (a dragged thumb).
pub type ThumbSet = fn (holder prim.Holder, offset f32, vertical bool)

// ThumbTarget is how a dragged thumb scrolls its container; a nil `set` makes the thumb
// display-only.
pub struct ThumbTarget {
pub:
	holder prim.Holder
	set    ThumbSet = unsafe { nil }
}

struct ThumbSeen {
	bounds   prim.Rect
	alpha    f32
	offset   f32
	total    f32
	vertical bool
	thumb    prim.Rect
	target   ThumbTarget
}

struct ThumbDrag {
	bounds   prim.Rect
	vertical bool
	start    f32 // the pointer along the axis when the drag began
	from     f32 // the offset then
}

struct ThumbState {
mut:
	seen []ThumbSeen // containers with a thumb in the last paint, shown or not
	prev []ThumbSeen // the paint before: an offset that changed reveals the thumb
	at   prim.Point  // where the last activity was
	ms   f64 = -1.0e9 // when
	drag ?ThumbDrag
	hot  bool // the pointer is on a thumb (its hover look)
}

fn (mut s ThumbState) next_paint() {
	s.prev, s.seen = s.seen, s.prev
	s.seen.clear()
}

// note_thumb_activity records a pointer move (wheel false) or a wheel turn at p. Moves
// count only over a scrolling container, so leaving one lets its thumb fade from there.
fn (mut w Window) note_thumb_activity(p prim.Point, wheel bool) {
	mut over, mut dim, mut hot := false, false, false // runs on every pointer move: no allocation
	for t in w.thumbs.seen {
		if t.bounds.contains(p) {
			over = true
			dim = dim || t.alpha < 1
			hot = hot || (t.alpha > 0 && thumb_grip(t.thumb, t.vertical).contains(p))
		}
	}
	if hot != w.thumbs.hot && w.thumb_style.hover.a > 0 {
		w.thumbs.hot = hot
		w.invalidate_paint() // the thumb under the pointer changes its look
	}
	if !wheel && !over {
		return
	}
	app := w.app
	w.thumbs.at, w.thumbs.ms = p, app.plat.now_ms()
	if dim {
		w.invalidate_paint() // a hidden or fading thumb comes back
	}
}

// thumb_grip is the 14 px track around a thumb that the pointer can take it by.
fn thumb_grip(thumb prim.Rect, vertical bool) prim.Rect {
	return if vertical {
		prim.rect(thumb.x - 4, thumb.y, 14, thumb.height)
	} else {
		prim.rect(thumb.x, thumb.y - 4, thumb.width, 14)
	}
}

fn along(p prim.Point, vertical bool) f32 {
	return if vertical { p.y } else { p.x }
}

// thumb_down starts dragging a shown thumb under p (its whole 14 px track is the grip).
fn (mut w Window) thumb_down(p prim.Point) bool {
	for i := w.thumbs.seen.len - 1; i >= 0; i-- { // painted last = on top
		t := w.thumbs.seen[i]
		grip := thumb_grip(t.thumb, t.vertical)
		if t.alpha > 0 && t.target.set != unsafe { nil } && grip.contains(p) {
			w.thumbs.drag = ThumbDrag{t.bounds, t.vertical, along(p, t.vertical), t.offset}
			return true
		}
	}
	return false
}

// thumb_move scrolls the dragged thumb's container: the pointer's travel over the track
// maps to the content's scroll range.
fn (mut w Window) thumb_move(p prim.Point) bool {
	d := w.thumbs.drag or { return false }
	for t in w.thumbs.seen {
		if t.bounds != d.bounds || t.vertical != d.vertical {
			continue
		}
		view := if t.vertical { t.bounds.height } else { t.bounds.width }
		track := view - 8 - (if t.vertical { t.thumb.height } else { t.thumb.width })
		if track > 0 {
			range := t.total - view
			off := prim.clamp(d.from + (along(p, t.vertical) - d.start) * range / track, 0, range)
			t.target.set(t.target.holder, off, t.vertical)
			w.invalidate_prepaint()
		}
		break
	}
	return true
}

fn (mut w Window) thumb_up() bool {
	if w.thumbs.drag == none {
		return false
	}
	w.thumbs.drag = none
	return true
}

// scroll_thumb paints the thumb of a container whose `bounds` show part of `total` px of
// content scrolled by `offset`, along the vertical axis or the horizontal one: a 6 px pill
// inset 4 from the edge, at least 25 long, in the window's scroll color.
pub fn (mut cx PaintCx) scroll_thumb(bounds prim.Rect, total f32, offset f32, vertical bool, target ThumbTarget) {
	cx.scroll_thumb_with(bounds, total, offset, vertical, target, cx.win.thumb_style)
}

// scroll_thumb_with is scroll_thumb in a style of its own (a terminal's).
pub fn (mut cx PaintCx) scroll_thumb_with(bounds prim.Rect, total f32, offset f32, vertical bool, target ThumbTarget, st ThumbStyle) {
	view := if vertical { bounds.height } else { bounds.width }
	if total <= view + 0.5 {
		return
	}
	mut w := cx.win
	now := cx.now_ms()
	for p in w.thumbs.prev {
		if p.bounds == bounds && p.vertical == vertical && p.offset != offset {
			w.thumbs.at, w.thumbs.ms = bounds.center(), now // moved: keys, a drag, the app
			break
		}
	}
	mut a := f32(0)
	if st.always {
		a = 1
	} else if bounds.contains(w.thumbs.at) {
		e := now - w.thumbs.ms
		if e < st.hold_ms {
			a = 1
			cx.request_paint_after(int(st.hold_ms - e) + 1)
		} else if e < st.hold_ms + st.fade_ms {
			a = f32(1 - (e - st.hold_ms) / st.fade_ms)
			cx.request_paint_after(16)
		}
	}
	len := prim.max(st.min_len, view * view / total - 8)
	pos := 4 + prim.clamp(offset / (total - view), 0, 1) * (view - 8 - len)
	mut r := if vertical {
		prim.rect(bounds.right() - st.inset - st.width, bounds.y + pos, st.width, len)
	} else {
		prim.rect(bounds.x + pos, bounds.bottom() - st.inset - st.width, len, st.width)
	}
	mut c := if st.color.a > 0 { st.color } else { w.scroll_color }
	mut dragged := false
	if d := w.thumbs.drag {
		dragged = d.bounds == bounds && d.vertical == vertical
	}
	if st.hover.a > 0 && a > 0 && (thumb_grip(r, vertical).contains(w.mouse) || dragged) {
		c = st.hover
		grow := st.wide - st.width
		r = if vertical {
			prim.rect(r.x - grow, r.y, st.wide, r.height)
		} else {
			prim.rect(r.x, r.y - grow, r.width, st.wide)
		}
	}
	w.thumbs.seen << ThumbSeen{bounds, a, offset, total, vertical, r, target}
	if a <= 0 {
		return
	}
	cx.quad(gpu.Quad{
		bounds:     r
		background: c.alpha(c.a * a)
		radii:      prim.corners(st.radius)
	})
}
