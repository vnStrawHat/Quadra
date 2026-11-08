// Flowchart placement: node sizes from their text and shape, positions across the ranks
// by repeated median alignment (each node moves toward the median of its neighbours, then
// the rank is spread out again keeping its order), positions along the ranks by thickness.
module mermaid

import quadra.prim

fn node_size(shape Shape, tb TextBox) (f32, f32) {
	w, h := tb.width + 32, prim.max(tb.height + 20, 38)
	return match shape {
		.rect, .round { w, h }
		.stadium { tb.width + h + 12, h }
		.circle { prim.max(tb.width, tb.height) + 28, prim.max(tb.width, tb.height) + 28 }
		.diamond { tb.width + tb.height + 30, tb.width + tb.height + 30 }
		.hexagon { w + h / 2, h }
		.subroutine { w + 16, h }
		.cylinder { w, h + 14 }
		.flag { w + h / 4, h }
	}
}

fn median(vals []f32) f32 {
	mut v := vals.clone()
	v.sort()
	n := v.len
	return if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2 }
}

// sep is the distance between the centers of neighbours a and b in a rank.
fn (g &Layers) sep(a int, b int, vertical bool) f32 {
	na, nb := g.n[a], g.n[b]
	gap := if na.real >= 0 && nb.real >= 0 {
		if vertical { f32(40) } else { 26 }
	} else if na.real >= 0 || nb.real >= 0 {
		f32(18)
	} else {
		f32(12)
	}
	c := common(na.path, nb.path)
	// Frames take 14 px a side; across the ranks of a left-right chart, also their title.
	frame := if vertical { f32(18) } else { 40 }
	return (na.w + nb.w) / 2 + gap + frame * f32(na.path.len + nb.path.len - 2 * c)
}

// spread places a rank's nodes as close to want as their order and separation allow:
// the mean of a left-packed and a right-packed placement, both feasible, so it is too.
fn (mut g Layers) spread(row []int, want []f32, vertical bool) {
	n := row.len
	if n == 0 {
		return
	}
	mut l := want.clone()
	for i in 1 .. n {
		l[i] = prim.max(want[i], l[i - 1] + g.sep(row[i - 1], row[i], vertical))
	}
	mut r := want.clone()
	for i := n - 2; i >= 0; i-- {
		r[i] = prim.min(want[i], r[i + 1] - g.sep(row[i], row[i + 1], vertical))
	}
	for i, v in row {
		g.n[v].pos = (l[i] + r[i]) / 2
	}
}

fn (mut g Layers) place(vertical bool) {
	for row in g.rows {
		mut x := f32(0)
		for i, v in row {
			if i > 0 {
				x += g.sep(row[i - 1], v, vertical)
			}
			g.n[v].pos = x
		}
	}
	for it in 0 .. 12 {
		mode := it % 3 // 0: toward the rank above, 1: below, 2: both
		for k in 0 .. g.rows.len {
			r := if mode == 1 { g.rows.len - 1 - k } else { k }
			row := g.rows[r]
			mut want := []f32{cap: row.len}
			for v in row {
				mut nb := []f32{}
				if mode != 1 {
					for w in g.n[v].up {
						nb << g.n[w].pos
					}
				}
				if mode != 0 {
					for w in g.n[v].down {
						nb << g.n[w].pos
					}
				}
				want << if nb.len > 0 { median(nb) } else { g.n[v].pos }
			}
			g.spread(row, want, vertical)
		}
	}
	mut lo := f32(1e9)
	for nd in g.n {
		lo = prim.min(lo, nd.pos - nd.w / 2)
	}
	for mut nd in g.n {
		nd.pos -= lo
	}
}

// layout_flow lays out a flowchart and returns what to paint.
fn layout_flow(f &Flow, mut m Measure) Drawing {
	size := m.base.size
	mut tbs := []TextBox{cap: f.nodes.len}
	for nd in f.nodes {
		tbs << m.box(nd.label, size, 400)
	}
	mut lbs := []TextBox{cap: f.edges.len}
	for e in f.edges {
		lbs << m.box(e.label, size * 0.92, 400)
	}
	vertical := f.dir in ['TD', 'TB', 'BT']
	mut g := layers(f)
	for k, chain in g.chains {
		if lbs[k].lines.len > 0 && chain.len > 2 {
			mid := chain[(chain.len - 1) / 2]
			lw, lh := lbs[k].width + 10, lbs[k].height + 4
			g.n[mid].w, g.n[mid].h = if vertical { lw, lh } else { lh, lw }
		}
	}
	mut sizes := []prim.Size{cap: f.nodes.len}
	for v, nd in f.nodes {
		w, h := node_size(nd.shape, tbs[v])
		sizes << prim.Size{w, h}
		g.n[v].w, g.n[v].h = if vertical { w, h } else { h, w }
	}
	g.order()
	g.place(vertical)
	// Along the ranks: each rank as thick as its thickest node, half a gap on each side.
	half := if vertical { f32(26) } else { 30 }
	mut start := []f32{len: g.rows.len}
	mut thick := []f32{len: g.rows.len}
	before, after := frame_room(f, g, vertical, m)
	mut cur := f32(0)
	for r, row in g.rows {
		for v in row {
			thick[r] = prim.max(thick[r], g.n[v].h)
		}
		cur += before[r]
		start[r] = cur
		cur += thick[r] + half + after[r]
	}
	total := cur - half
	mut centers := []prim.Point{cap: g.n.len}
	for nd in g.n {
		along := start[nd.rank] + thick[nd.rank] / 2
		centers << match f.dir {
			'BT' { prim.pt(nd.pos, total - along) }
			'LR' { prim.pt(along, nd.pos) }
			'RL' { prim.pt(total - along, nd.pos) }
			else { prim.pt(nd.pos, along) }
		}
	}
	mut boxes := []prim.Rect{cap: f.nodes.len}
	for v in 0 .. f.nodes.len {
		c, s := centers[v], sizes[v]
		boxes << prim.rect(c.x - s.width / 2, c.y - s.height / 2, s.width, s.height)
	}
	mut d := Drawing{}
	d.frames(f, boxes, m)
	d.edges(f, g, centers, boxes, lbs, m)
	for v, nd in f.nodes {
		d.shape(nd.shape, boxes[v])
		dy := if nd.shape == .cylinder { f32(5) } else { 0 }
		d.text_ops(m, tbs[v], boxes[v].x + boxes[v].width / 2,
			boxes[v].y + boxes[v].height / 2 + dy, .text)
	}
	d.normalize(8)
	return d
}

// frame_room is the extra space along the ranks before and after each rank for the
// subgraph frames that start or end there (their padding and, top-down, their title).
fn frame_room(f &Flow, g &Layers, vertical bool, m &Measure) ([]f32, []f32) {
	mut before := []f32{len: g.rows.len}
	mut after := []f32{len: g.rows.len}
	title_h := m.line_h(m.base.size * 0.85) + 6
	for s in 0 .. f.subs.len {
		mut lo, mut hi := -1, -1
		for nd in g.n {
			if nd.real >= 0 && s in nd.path {
				lo = if lo < 0 || nd.rank < lo { nd.rank } else { lo }
				hi = if nd.rank > hi { nd.rank } else { hi }
			}
		}
		if lo < 0 {
			continue
		}
		top, bottom := if f.dir == 'BT' || f.dir == 'RL' { hi, lo } else { lo, hi }
		before[top] += 18 + if vertical { title_h } else { 0 }
		after[bottom] += 18
	}
	if f.dir == 'BT' || f.dir == 'RL' { // ranks run backwards: before and after swap
		return after, before
	}
	return before, after
}
