// Grid layout: placement, track sizing (columns before rows), alignment.
// ponytail: fr tracks have a 0 minimum (not min-content) and spanning items
// spread extra size evenly over their non-fixed tracks.
module layout

import quadra.prim

struct GridItem {
mut:
	id   NodeId
	row  int
	col  int
	rows int
	cols int
}

fn (mut t Tree) layout_grid(id NodeId, s &Style, kw f32, kh f32, aw f32, ah f32, perform bool) prim.Size {
	f := s.frame()
	g := s.grid_style()
	inner_w := inner_of(kw, f.horizontal())
	inner_h := inner_of(kh, f.vertical())
	avail_w := if inner_w != unknown { inner_w } else { inner_of(aw, f.horizontal()) }
	avail_h := if inner_h != unknown { inner_h } else { inner_of(ah, f.vertical()) }
	mut columns := g.columns.clone()
	if g.fill_min > 0 {
		n := if avail_w < unbounded {
			prim.max(1, (avail_w + s.gap_col) / (g.fill_min + s.gap_col))
		} else {
			1
		}
		columns = repeat(int(n), minmax(size_px(g.fill_min), size_fr(1)))
	}
	children := t.nodes[id].children.clone()
	mut absolutes := []NodeId{}
	mut flow := []NodeId{}
	for c in children {
		cs := t.nodes[c].style
		if cs.display == .none {
			t.nodes[c].rect = prim.Rect{}
		} else if cs.position == .absolute {
			absolutes << c
		} else {
			flow << c
		}
	}
	mut items := t.place_grid_items(flow, g, columns.len, g.rows.len)
	mut ncols := columns.len
	mut nrows := g.rows.len
	for it in items {
		ncols = int(prim.max(ncols, it.col + it.cols))
		nrows = int(prim.max(nrows, it.row + it.rows))
	}
	for columns.len < ncols {
		columns << g.auto_cols
	}
	mut rows := g.rows.clone()
	for rows.len < nrows {
		rows << g.auto_rows
	}
	colw := t.size_tracks(columns, items, true, inner_w, avail_w, s.gap_col, []f32{})
	rowh := t.size_tracks(rows, items, false, inner_h, avail_h, s.gap_row, colw)
	used_w := sum_tracks(colw, s.gap_col)
	used_h := sum_tracks(rowh, s.gap_row)
	out_w := if kw != unknown {
		kw
	} else {
		clamp_axis(used_w + f.horizontal(), s.min_width.resolve(aw), s.max_width.resolve(aw))
	}
	out_h := if kh != unknown {
		kh
	} else {
		clamp_axis(used_h + f.vertical(), s.min_height.resolve(ah), s.max_height.resolve(ah))
	}
	if perform {
		colx := track_offsets(colw, s.gap_col, out_w - f.horizontal() - used_w, g.justify_content)
		rowy := track_offsets(rowh, s.gap_row, out_h - f.vertical() - used_h, g.align_content)
		for mut it in items {
			cell := prim.rect(f.left + colx[it.col], f.top + rowy[it.row], span_size(colw, it.col,
				it.cols, s.gap_col), span_size(rowh, it.row, it.rows, s.gap_row))
			t.place_in_cell(it.id, cell, g.justify_items, s.align_items)
		}
		mut extent := prim.Size{}
		for it in items {
			r := t.nodes[it.id].rect
			extent.width = prim.max(extent.width, r.right())
			extent.height = prim.max(extent.height, r.bottom())
		}
		t.nodes[id].content = prim.size(extent.width + s.padding.right, extent.height +
			s.padding.bottom)
		t.place_absolutes(absolutes, s, out_w, out_h)
	}
	return prim.size(out_w, out_h)
}

fn (mut t Tree) place_grid_items(ids []NodeId, g GridStyle, ncols_def int, nrows_def int) []GridItem {
	areas := parse_areas(g.areas)
	ncols := int(prim.max(1, if ncols_def > 0 { ncols_def } else { 1 }))
	mut taken := map[u64]bool{}
	mut items := []GridItem{}
	mut pending := []GridItem{}
	for c in ids {
		cg := t.nodes[c].style.grid_style()
		mut it := GridItem{c, cg.row_start - 1, cg.col_start - 1, int(prim.max(1, cg.row_span)), int(prim.max(1,
			cg.col_span))}
		if cg.area != '' {
			if a := areas[cg.area] {
				it = GridItem{c, a.row, a.col, a.rows, a.cols}
			}
		}
		if it.row >= 0 && it.col >= 0 {
			mark(mut taken, it)
			items << it
		} else {
			pending << it
		}
	}
	mut cr, mut cc := 0, 0
	for mut it in pending {
		if g.dense {
			cr, cc = 0, 0
		}
		for {
			r := if it.row >= 0 { it.row } else { cr }
			col := if it.col >= 0 { it.col } else { cc }
			fits := (g.column_flow || col + it.cols <= ncols || it.col >= 0)
				&& is_free(taken, r, col, it)
			if fits {
				it.row, it.col = r, col
				break
			}
			if g.column_flow {
				cr++
				if cr >= int(prim.max(1, nrows_def)) && it.row < 0 {
					cr = 0
					cc++
				}
			} else {
				cc++
				if cc + it.cols > ncols && it.col < 0 {
					cc = 0
					cr++
				}
			}
		}
		mark(mut taken, it)
		items << it
		if g.column_flow {
			cr = it.row + it.rows
		} else {
			cc = it.col + it.cols
		}
	}
	return items
}

fn cell_key(r int, c int) u64 {
	return (u64(r) << 32) | u64(u32(c))
}

fn is_free(taken map[u64]bool, r int, c int, it GridItem) bool {
	for dr in 0 .. it.rows {
		for dc in 0 .. it.cols {
			if taken[cell_key(r + dr, c + dc)] {
				return false
			}
		}
	}
	return true
}

fn mark(mut taken map[u64]bool, it GridItem) {
	for dr in 0 .. it.rows {
		for dc in 0 .. it.cols {
			taken[cell_key(it.row + dr, it.col + dc)] = true
		}
	}
}
