// Flowchart drawing: subgraph frames, edges along their dummy nodes as smooth curves that
// end on the node outlines, edge labels, and node shapes.
module mermaid

import math
import quadra.prim

struct SubDepth {
	sub   int
	depth int
}

// frames draws each subgraph as a titled frame around its nodes and inner frames.
fn (mut d Drawing) frames(f &Flow, boxes []prim.Rect, m &Measure) {
	if f.subs.len == 0 {
		return
	}
	mut deep := []SubDepth{cap: f.subs.len}
	for i in 0 .. f.subs.len {
		deep << SubDepth{i, sub_path(f, i).len}
	}
	deep.sort(a.depth > b.depth) // innermost first: outer frames wrap inner ones
	order := deep.map(it.sub)
	title_size := m.base.size * 0.85
	title_h := m.line_h(title_size) + 6
	mut rects := []prim.Rect{len: f.subs.len}
	mut have := []bool{len: f.subs.len}
	for s in order {
		mut r := prim.Rect{}
		mut any := false
		for v, nd in f.nodes {
			if nd.sub == s {
				r = if any { r.union(boxes[v]) } else { boxes[v] }
				any = true
			}
		}
		for c in 0 .. f.subs.len {
			if f.subs[c].parent == s && have[c] {
				r = if any { r.union(rects[c]) } else { rects[c] }
				any = true
			}
		}
		if !any {
			continue
		}
		rects[s] = prim.rect(r.x - 14, r.y - 14 - title_h, r.width + 28, r.height + 28 + title_h)
		have[s] = true
	}
	for i := order.len - 1; i >= 0; i-- {
		s := order[i]
		if !have[s] {
			continue
		}
		r := rects[s]
		d.ops << Op{
			kind:   .box
			rect:   r
			radius: 6
			bg:     .frame
			border: .line
		}
		d.text_left(m, f.subs[s].title, r.x + 10, r.y + 4 + title_h / 2, title_size, 600, .text_2)
	}
}

// edges draws every edge: its curve, heads and label.
fn (mut d Drawing) edges(f &Flow, g &Layers, centers []prim.Point, boxes []prim.Rect, lbs []TextBox, m &Measure) {
	for k, e in f.edges {
		width := if e.stroke == .thick { f32(2.5) } else { 1.5 }
		mut pts := []prim.Point{}
		if e.from == e.to {
			b := boxes[e.from]
			r, cy := b.right(), b.y + b.height / 2
			pts = basis([prim.pt(r, cy - 8), prim.pt(r + 26, cy - 14),
				prim.pt(r + 26, cy + 14), prim.pt(r, cy + 8)])
		} else {
			mut c := g.chains[k].map(centers[it])
			if g.rev[k] {
				c = reversed(c)
			}
			c[0] = clip(boxes[e.from], f.nodes[e.from].shape, c[1])
			c[c.len - 1] = clip(boxes[e.to], f.nodes[e.to].shape, c[c.len - 2])
			pts = basis(c)
		}
		if pts.len < 2 {
			continue
		}
		tip, start := pts.last(), pts[0]
		pts = cut_end(pts, head_len(e.head, width))
		if e.tail != .none {
			pts = reversed(cut_end(reversed(pts), head_len(e.tail, width)))
		}
		d.poly(pts, width, e.stroke == .dotted, .accent)
		d.head(tip, pts.last(), e.head, .accent)
		d.head(start, pts[0], e.tail, .accent)
		if lbs[k].lines.len > 0 {
			chain := g.chains[k]
			lc := if chain.len > 2 { centers[chain[(chain.len - 1) / 2]] } else { pts[pts.len / 2] }
			tb := lbs[k]
			d.ops << Op{
				kind:   .box
				rect:   prim.rect(lc.x - tb.width / 2 - 5, lc.y - tb.height / 2 - 2, tb.width + 10,

					tb.height + 4)
				radius: 3
				bg:     .canvas
			}
			d.text_ops(m, tb, lc.x, lc.y, .text)
		}
	}
}

// shape draws a node's outline and fill in r.
fn (mut d Drawing) shape(shape Shape, r prim.Rect) {
	x, y, w, h := r.x, r.y, r.width, r.height
	cx, cy := x + w / 2, y + h / 2
	match shape {
		.rect, .round, .stadium, .circle, .subroutine {
			radius := match shape {
				.round { f32(10) }
				.stadium, .circle { h / 2 }
				.subroutine { f32(0) }
				else { f32(4) }
			}

			d.ops << Op{
				kind:   .box
				rect:   r
				radius: radius
				bg:     .panel
				border: .line
			}
			if shape == .subroutine {
				d.poly([prim.pt(x + 8, y), prim.pt(x + 8, y + h)], 1, false, .line)
				d.poly([prim.pt(x + w - 8, y), prim.pt(x + w - 8, y + h)], 1, false, .line)
			}
		}
		.diamond {
			d.fill([prim.pt(cx, y), prim.pt(x + w, cy), prim.pt(cx, y + h),
				prim.pt(x, cy)])
		}
		.hexagon {
			k := h / 4
			d.fill([prim.pt(x + k, y), prim.pt(x + w - k, y),
				prim.pt(x + w, cy), prim.pt(x + w - k, y + h),
				prim.pt(x + k, y + h), prim.pt(x, cy)])
		}
		.flag {
			d.fill([prim.pt(x, y), prim.pt(x + w, y), prim.pt(x + w, y + h),
				prim.pt(x, y + h), prim.pt(x + h / 4, cy)])
		}
		.cylinder {
			ry := f32(7)
			mut out := []prim.Point{}
			for i in 0 .. 17 { // top back half, right to left
				a := math.pi * f64(i) / 16
				out << prim.pt(cx + w / 2 * f32(math.cos(a)), y + ry - ry * f32(math.sin(a)))
			}
			for i in 0 .. 17 { // bottom front half, left to right
				a := math.pi * f64(i) / 16
				out << prim.pt(cx - w / 2 * f32(math.cos(a)), y + h - ry + ry * f32(math.sin(a)))
			}
			d.fill(out)
			mut rim := []prim.Point{}
			for i in 0 .. 17 {
				a := math.pi * f64(i) / 16
				rim << prim.pt(cx - w / 2 * f32(math.cos(a)), y + ry + ry * f32(math.sin(a)))
			}
			d.poly(rim, 1, false, .line)
		}
	}
}

fn (mut d Drawing) fill(pts []prim.Point) {
	d.ops << Op{
		kind:   .fill
		pts:    pts
		bg:     .panel
		border: .line
		closed: true
	}
}
