// Flexible length resolution, main-axis placement and absolute positioning.
module layout

import quadra.prim

// resolve_flexible distributes free space by grow (or removes it by shrink × base),
// freezing items that hit their min/max until the result is stable.
// On overflow, items that grow give space back first; fixed-size siblings (a header above a
// scrolling body) shrink only if that is not enough. This differs from CSS on purpose: it
// removes the need for `min-height: 0` style fixes in every app shell.
fn resolve_flexible(mut items []FlexItem, l FlexLine, space f32, gap f32) {
	n := l.end - l.start
	if n == 0 {
		return
	}
	mut fixed := gap * f32(n - 1)
	for j in l.start .. l.end {
		fixed += items[j].m_main_a + items[j].m_main_b
	}
	free := space - line_outer(items, l, gap, true)
	growing := free > 0
	mut flexible_first := false
	if !growing {
		for j in l.start .. l.end {
			flexible_first = flexible_first || (items[j].grow > 0 && items[j].shrink > 0)
		}
	}
	for j in l.start .. l.end {
		mut it := unsafe { &items[j] }
		it.main = it.hyp
		it.frozen = free == 0 || (growing && it.grow == 0)
			|| (!growing && it.shrink == 0) || (flexible_first && it.grow == 0)
	}
	distribute(mut items, l, space - fixed, growing)
	if flexible_first && space - line_outer(items, l, gap, false) < -0.001 {
		for j in l.start .. l.end {
			mut it := unsafe { &items[j] }
			it.frozen = it.shrink == 0 || it.grow > 0
		}
		distribute(mut items, l, space - fixed, false)
	}
}

// distribute runs the freeze loop over the unfrozen items of a line.
fn distribute(mut items []FlexItem, l FlexLine, inner f32, growing bool) {
	for _ in 0 .. l.end - l.start + 1 {
		mut rem := inner
		mut factor := f32(0)
		for j in l.start .. l.end {
			it := items[j]
			if it.frozen {
				rem -= it.main
			} else {
				rem -= it.base
				factor += if growing { it.grow } else { it.shrink * it.base }
			}
		}
		if factor == 0 {
			break
		}
		mut total := f32(0)
		for j in l.start .. l.end {
			mut it := unsafe { &items[j] }
			if it.frozen {
				continue
			}
			target := if growing {
				it.base + rem * it.grow / factor
			} else {
				it.base + rem * it.shrink * it.base / factor
			}
			clamped := prim.max(0, clamp_axis(target, it.min_main, it.max_main))
			it.violation = clamped - target
			total += it.violation
			it.main = clamped
		}
		if prim.max(total, -total) < 0.001 {
			break
		}
		for j in l.start .. l.end {
			mut it := unsafe { &items[j] }
			if !it.frozen && ((total > 0 && it.violation > 0) || (total < 0 && it.violation < 0)) {
				it.frozen = true
			}
		}
	}
}

// place_line positions items along the main axis (justify, auto margins) and inside the line.
fn place_line(mut items []FlexItem, l FlexLine, space f32, gap f32, justify Justify, reverse bool) {
	n := l.end - l.start
	if n == 0 {
		return
	}
	free := space - line_outer(items, l, gap, false)
	mut autos := 0
	for j in l.start .. l.end {
		autos += int(items[j].auto_ma) + int(items[j].auto_mb)
	}
	mut lead := f32(0)
	mut between := gap
	auto_share := if autos > 0 && free > 0 { free / f32(autos) } else { 0 }
	if autos == 0 {
		match justify {
			.start {}
			.end {
				lead = free
			}
			.center {
				lead = free / 2
			}
			.between {
				if n > 1 && free > 0 {
					between += free / f32(n - 1)
				}
			}
			.around {
				if free > 0 {
					lead = free / f32(n) / 2
					between += free / f32(n)
				}
			}
			.evenly {
				if free > 0 {
					lead = free / f32(n + 1)
					between += free / f32(n + 1)
				}
			}
		}
	}
	mut pos := lead
	for j in l.start .. l.end {
		mut it := unsafe { &items[j] }
		pos += it.m_main_a + if it.auto_ma { auto_share } else { 0 }
		it.pos_main = if reverse { space - pos - it.main } else { pos }
		pos += it.main + it.m_main_b + (if it.auto_mb { auto_share } else { 0 }) + between
		free_c := l.cross - it.cross - it.m_cross_a - it.m_cross_b
		offset := if it.auto_ca && it.auto_cb {
			free_c / 2
		} else if it.auto_ca {
			free_c
		} else if it.auto_cb {
			f32(0)
		} else {
			match it.align {
				.start, .stretch { f32(0) }
				.end { free_c }
				.center { free_c / 2 }
			}
		}
		it.pos_cross = l.pos + it.m_cross_a + offset
	}
}

// place_absolutes lays out absolutely positioned children against the padding box.
fn (mut t Tree) place_absolutes(ids []NodeId, s &Style, outer_w f32, outer_h f32) {
	b := s.border
	pbw := prim.max(0, outer_w - b.horizontal())
	pbh := prim.max(0, outer_h - b.vertical())
	for c in ids {
		cs := t.nodes[c].style
		m := cs.margin
		left := cs.inset_left.resolve(pbw)
		right := cs.inset_right.resolve(pbw)
		top := cs.inset_top.resolve(pbh)
		bottom := cs.inset_bottom.resolve(pbh)
		mut w := cs.width.resolve(pbw)
		mut h := cs.height.resolve(pbh)
		if w == unknown && left != unknown && right != unknown {
			w = prim.max(0, pbw - left - right - m.horizontal())
		}
		if h == unknown && top != unknown && bottom != unknown {
			h = prim.max(0, pbh - top - bottom - m.vertical())
		}
		if w == unknown || h == unknown {
			sz := t.layout_node(c, w, h, pbw, pbh, false)
			w, h = sz.width, sz.height
		}
		t.layout_node(c, w, h, w, h, true)
		x := if left != unknown {
			b.left + left + m.left
		} else if right != unknown {
			outer_w - b.right - right - w - m.right
		} else {
			s.frame().left + m.left
		}
		y := if top != unknown {
			b.top + top + m.top
		} else if bottom != unknown {
			outer_h - b.bottom - bottom - h - m.bottom
		} else {
			s.frame().top + m.top
		}
		t.nodes[c].rect = prim.rect(x, y, w, h)
	}
}
