// Strokes on the GPU: a polyline becomes one `line` instance per segment, antialiased exactly
// in the pixel shader (distance to the segment, round joins and caps). A pixel near a joint
// is drawn only by the nearest of the segment and its two neighbours, so joints are never
// blended twice and translucent strokes stay even. Nothing goes through the atlas, so
// animated strokes (spinners, canvas) cost no memory.
module gpu

import quadra.prim

// Line instance flags in stroke mode (params.y == 3).
pub const seg_prev = u32(1) // a segment comes before this one (its start is uv.xy)

pub const seg_next = u32(2) // a segment follows this one (its end is uv.zw)

pub const seg_cont = u32(4) // continues the stroke of the previous instance (order.v)

// stroke appends a polyline of width w (device px). closed joins the last point to the first.
pub fn (mut s Scene) stroke(pts []prim.Point, w f32, color prim.Color, closed bool, mask ContentMask) {
	// A closed polyline gets a last segment back to the first point (unless it repeats it).
	m := if closed && pts.len > 2 && pts[0] != pts.last() { pts.len } else { pts.len - 1 }
	if m < 1 || w <= 0 || color.is_transparent() {
		return
	}
	ring := closed && pts.len > 2
	pad := w / 2 + 1
	c := color.to_array()
	for k in 0 .. m {
		a, b := pts[k], pts[(k + 1) % pts.len]
		x0, y0 := prim.min(a.x, b.x) - pad, prim.min(a.y, b.y) - pad
		x1, y1 := prim.max(a.x, b.x) + pad, prim.max(a.y, b.y) + pad
		mut i := base(.line, prim.rect(x0, y0, x1 - x0, y1 - y0), mask)
		i.color = c
		i.params[0] = w
		i.params[1] = 3
		i.radii = [a.x, a.y, b.x, b.y]!
		if k > 0 || ring {
			p := pts[(k + m - 1) % m]
			i.uv[0], i.uv[1] = p.x, p.y
			i.flags |= seg_prev
		}
		if k < m - 1 || ring {
			q := if k < m - 1 { pts[(k + 2) % pts.len] } else { pts[(k + 2) % m] } // ring: wraps to P1
			i.uv[2], i.uv[3] = q.x, q.y
			i.flags |= seg_next
		}
		if k > 0 {
			i.flags |= seg_cont
		}
		s.add(i)
	}
}
