// Device pixel snapping for quads and clips (specs/08, Device pixels).
module core

import math
import quadra.prim

// snaps: quads go on the device pixel grid unless a rotation or a scale is in effect
// (specs/08, Device pixels).
fn (cx &PaintCx) snaps() bool {
	return cx.xf.k == 1 && cx.xf.id == 0
}

// snap_px rounds a device coordinate to a whole pixel, ties toward zero.
fn snap_px(v f32) f32 {
	return if v >= 0 { f32(math.ceil(v - 0.5)) } else { -f32(math.ceil(-v - 0.5)) }
}

// snap_stroke rounds a device border width like snap_px, at least 1 when it is above 0.
fn snap_stroke(w f32) f32 {
	return if w <= 0 { 0 } else { math.max(f32(1), snap_px(w)) }
}

// snapped_width is a stroke width as quad paints it (whole device pixels), in logical px.
pub fn (cx &PaintCx) snapped_width(w f32) f32 {
	return if cx.snaps() { snap_stroke(w * cx.scale) / cx.scale } else { w }
}

// snapped is r with its edges where quad paints them (whole device pixels), in logical px.
pub fn (cx &PaintCx) snapped(r prim.Rect) prim.Rect {
	if !cx.snaps() {
		return r
	}
	d, s, t := cx.dev(r), cx.scale, cx.xf.t
	x, y := snap_px(d.x), snap_px(d.y)
	return prim.rect(x / s - t.x, y / s - t.y, (snap_px(d.x + d.width) - x) / s, (snap_px(d.y +
		d.height) - y) / s)
}
