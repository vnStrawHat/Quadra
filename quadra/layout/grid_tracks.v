// Grid track sizing and alignment helpers.
module layout

import quadra.prim

// size_tracks resolves one axis. For rows, col_widths gives each item its final width.
fn (mut t Tree) size_tracks(tracks []Track, items []GridItem, columns bool, inner f32, avail f32, gap f32, col_widths []f32) []f32 {
	n := tracks.len
	mut sizes := []f32{len: n}
	mut fixed := []bool{len: n}
	for i, tr in tracks {
		v := fixed_size(tr.min, inner)
		w := fixed_size(tr.max, inner)
		if tr.min.unit in [.px, .pct] && tr.max.unit in [.px, .pct] {
			sizes[i] = w
			fixed[i] = true
		} else if v != unknown {
			sizes[i] = v
		}
	}
	// Content contributions: single-span items first, then spanning items.
	for pass in 0 .. 2 {
		for it in items {
			start := if columns { it.col } else { it.row }
			span := if columns { it.cols } else { it.rows }
			if (pass == 0) != (span == 1) {
				continue
			}
			mut free_tracks := []int{}
			for k in start .. start + span {
				if !fixed[k] && !(tracks[k].max.unit == .fr && inner != unknown) {
					free_tracks << k
				}
			}
			if free_tracks.len == 0 {
				continue
			}
			min_only := tracks[start].min.unit == .min_content
				|| tracks[start].max.unit == .min_content
			need := t.contribution(it, columns, min_only, col_widths, gap)
			have := span_size(sizes, start, span, gap)
			if need > have {
				extra := (need - have) / f32(free_tracks.len)
				for k in free_tracks {
					sizes[k] += extra
					sizes[k] = clamp_axis(sizes[k], unknown, fixed_size(tracks[k].max, inner))
				}
			}
		}
	}
	// fr tracks share the space left over, never below their minimum.
	if inner != unknown || avail < unbounded {
		space := if inner != unknown { inner } else { avail }
		mut used := gap * f32(prim.max(0, n - 1))
		mut frs := f32(0)
		for i, tr in tracks {
			if tr.max.unit == .fr {
				frs += tr.max.value
			} else {
				used += sizes[i]
			}
		}
		if frs > 0 && inner != unknown {
			unit := prim.max(0, space - used) / frs
			for i, tr in tracks {
				if tr.max.unit == .fr {
					sizes[i] = prim.max(sizes[i], unit * tr.max.value)
				}
			}
		}
	}
	return sizes
}

fn fixed_size(ts TrackSize, inner f32) f32 {
	return match ts.unit {
		.px {
			ts.value
		}
		.pct {
			if inner != unknown && inner < unbounded {
				ts.value / 100 * inner
			} else {
				unknown
			}
		}
		else {
			unknown
		}
	}
}

// contribution measures how much space an item needs along one axis.
fn (mut t Tree) contribution(it GridItem, columns bool, min_only bool, col_widths []f32, gap f32) f32 {
	m := t.nodes[it.id].style.margin
	if columns {
		sz := t.layout_node(it.id, unknown, unknown, if min_only { 0 } else { unbounded },
			unbounded, false)
		return sz.width + m.horizontal()
	}
	w := span_size(col_widths, it.col, it.cols, gap) - m.horizontal()
	sz := t.layout_node(it.id, prim.max(0, w), unknown, prim.max(0, w), unbounded, false)
	return sz.height + m.vertical()
}

fn span_size(sizes []f32, start int, span int, gap f32) f32 {
	mut s := f32(0)
	for k in start .. start + span {
		if k < sizes.len {
			s += sizes[k]
		}
	}
	return s + gap * f32(span - 1)
}

fn sum_tracks(sizes []f32, gap f32) f32 {
	return span_size(sizes, 0, sizes.len, gap)
}

fn track_offsets(sizes []f32, gap f32, free f32, justify Justify) []f32 {
	mut out := []f32{len: sizes.len + 1}
	mut pos := match justify {
		.center { prim.max(0, free / 2) }
		.end { prim.max(0, free) }
		else { f32(0) }
	}

	for i, sz in sizes {
		out[i] = pos
		pos += sz + gap
	}
	out[sizes.len] = pos
	return out
}

// place_in_cell sizes an item inside its cell (stretch by default) and positions it.
fn (mut t Tree) place_in_cell(id NodeId, cell prim.Rect, justify_items Align, align_items Align) {
	cs := t.nodes[id].style
	m := cs.margin
	jx := resolve_self(cs.grid_style().justify_self, justify_items)
	ay := resolve_self(cs.align_self, align_items)
	inner := cell.inset(m)
	mut w := cs.width.resolve(cell.width)
	mut h := cs.height.resolve(cell.height)
	if w == unknown && jx == .stretch {
		w = inner.width
	}
	if h == unknown && ay == .stretch {
		h = inner.height
	}
	if w == unknown || h == unknown {
		sz := t.layout_node(id, w, h, inner.width, inner.height, false)
		w, h = sz.width, sz.height
	}
	t.layout_node(id, w, h, w, h, true)
	x := inner.x + align_offset(jx, inner.width - w)
	y := inner.y + align_offset(ay, inner.height - h)
	t.nodes[id].rect = prim.rect(x, y, w, h)
}

fn resolve_self(s AlignSelf, parent Align) Align {
	return match s {
		.auto { parent }
		.stretch { Align.stretch }
		.start { Align.start }
		.end { Align.end }
		.center { Align.center }
	}
}

fn align_offset(a Align, free f32) f32 {
	return match a {
		.end { free }
		.center { free / 2 }
		else { f32(0) }
	}
}
