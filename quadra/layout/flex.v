// Flexbox layout (CSS flexbox restricted to the properties in style.v).
// ponytail: min-width/height default to 0 (not CSS min-content) and align-content is
// always `start`; components opt out of shrinking with shrink 0 where it matters.
module layout

import quadra.prim

struct FlexItem {
mut:
	id        NodeId
	base      f32
	hyp       f32
	main      f32
	cross     f32
	def_cross f32
	min_main  f32
	max_main  f32
	min_cross f32
	max_cross f32
	m_main_a  f32 // margin at main start
	m_main_b  f32
	m_cross_a f32
	m_cross_b f32
	auto_ma   bool
	auto_mb   bool
	auto_ca   bool
	auto_cb   bool
	grow      f32
	shrink    f32
	align     Align
	frozen    bool
	violation f32
	pos_main  f32
	pos_cross f32
}

struct FlexLine {
mut:
	start int
	end   int
	cross f32
	pos   f32
}

struct Axes {
	row          bool
	inner_w      f32
	inner_h      f32
	avail_w      f32
	avail_h      f32
	stretch_into f32 // cross size stretched items will get, when known up front
}

fn inner_of(v f32, frame f32) f32 {
	return if v == unknown {
		unknown
	} else if v >= unbounded {
		unbounded
	} else {
		prim.max(0, v - frame)
	}
}

fn (mut t Tree) layout_flex(id NodeId, s &Style, kw f32, kh f32, aw f32, ah f32, perform bool) prim.Size {
	f := s.frame()
	row := s.is_row()
	inner_w := inner_of(kw, f.horizontal())
	inner_h := inner_of(kh, f.vertical())
	avail_w := if inner_w != unknown { inner_w } else { inner_of(aw, f.horizontal()) }
	avail_h := if inner_h != unknown { inner_h } else { inner_of(ah, f.vertical()) }
	inner_main := if row { inner_w } else { inner_h }
	inner_cross := if row { inner_h } else { inner_w }
	avail_main := if row { avail_w } else { avail_h }
	gap_main := if row { s.gap_col } else { s.gap_row }
	gap_cross := if row { s.gap_row } else { s.gap_col }
	// A scrolling cross axis lets items exceed it, so stretching cannot skip measuring them.
	// Only scrolling counts: `hidden` only clips (specs/05), its size is still its content.
	clips_cross := if row { s.overflow_y == .scroll } else { s.overflow_x == .scroll }
	ax := Axes{row, inner_w, inner_h, avail_w, avail_h, if !s.wrap && !clips_cross {
		inner_cross
	} else {
		unknown
	}}

	// No copies on this hot path: layout never adds nodes or changes child lists.
	children := t.nodes[id].children
	// Items and lines live in scratch kept per nesting depth: children laid out below use
	// the next depth, so nothing here is overwritten while in use.
	depth := t.depth
	t.depth++
	if depth == t.flex_items.len {
		t.flex_items << []FlexItem{}
		t.flex_lines << []FlexLine{}
	}
	mut items := unsafe { t.flex_items[depth] }
	items.clear()
	mut flines := unsafe { t.flex_lines[depth] }
	flines.clear()
	mut absolutes := []NodeId{}
	// The only growing item of a container with a definite main size ends at the leftover
	// space whatever its content (it grows into free space and, flexible first, gives back
	// overflow), so its base size is never measured: the app shell's body, an editor.
	scroll_main := (row && s.overflow_x == .scroll) || (!row && s.overflow_y == .scroll)
	mut sole := -1
	if inner_main != unknown && !s.wrap && !scroll_main {
		for c in children {
			cs := unsafe { &t.nodes[c].style }
			if cs.display != .none && cs.position != .absolute && cs.grow > 0 {
				sole = if sole == -1 { int(c) } else { -2 }
			}
		}
	}
	for c in children {
		t.nodes[c].parent = id // set here, where the child is touched anyway (relayout.v)
		cs := unsafe { &t.nodes[c].style }
		if cs.display == .none {
			t.nodes[c].rect = prim.Rect{}
			continue
		}
		if cs.position == .absolute {
			absolutes << c
			continue
		}
		items << t.flex_item(c, cs, &ax, s.align_items, int(c) == sole)
		// Items of a scroll container keep their size and overflow instead of shrinking.
		if (row && s.overflow_x == .scroll) || (!row && s.overflow_y == .scroll) {
			items[items.len - 1].shrink = 0
		}
	}

	collect_lines(items, s.wrap, if inner_main != unknown {
		inner_main
	} else {
		avail_main
	}, gap_main, mut flines)
	// Main size of an auto container: its widest line. Width is limited by the available space
	// (so text can wrap), and so is a scroll container's height (it scrolls instead); any other
	// auto height is the content height, so tall content never squeezes its own children.
	mut content_main := f32(0)
	for l in flines {
		content_main = prim.max(content_main, line_outer(items, l, gap_main, true))
	}
	clamp_main := row || s.overflow_y == .scroll // a column's main axis is y
	mut space := inner_main
	if space == unknown {
		space = if clamp_main && avail_main < unbounded {
			prim.min(content_main, avail_main)
		} else {
			content_main
		}
		own_min := if row { s.min_width.resolve(aw) } else { s.min_height.resolve(ah) }
		own_max := if row { s.max_width.resolve(aw) } else { s.max_height.resolve(ah) }
		frame_main := if row { f.horizontal() } else { f.vertical() }
		space = prim.max(0, clamp_axis(space + frame_main, own_min, own_max) - frame_main)
	}
	for l in flines {
		resolve_flexible(mut items, l, space, gap_main)
	}

	// Cross sizes: measured with the final main size, then stretched to the line.
	for mut it in items {
		if it.def_cross != unknown {
			it.cross = clamp_axis(it.def_cross, it.min_cross, it.max_cross)
		} else if it.align == .stretch && !it.auto_ca && !it.auto_cb && ax.stretch_into != unknown {
			// The size is decided by the stretch below; measuring it would be wasted work.
			it.cross = clamp_axis(prim.max(0, ax.stretch_into - it.m_cross_a - it.m_cross_b),
				it.min_cross, it.max_cross)
		} else {
			kw2 := if row { it.main } else { unknown }
			kh2 := if row { unknown } else { it.main }
			aw2 := if row { it.main } else { inner_of(avail_w, it.m_cross_a + it.m_cross_b) }
			ah2 := if row { inner_of(avail_h, it.m_cross_a + it.m_cross_b) } else { it.main }
			sz := t.layout_node(it.id, kw2, kh2, aw2, ah2, false)
			it.cross = clamp_axis(if row { sz.height } else { sz.width }, it.min_cross,
				it.max_cross)
		}
	}
	mut total_cross := f32(0)
	for i, mut l in flines {
		for j in l.start .. l.end {
			l.cross = prim.max(l.cross, items[j].cross + items[j].m_cross_a + items[j].m_cross_b)
		}
		if flines.len == 1 && inner_cross != unknown {
			// A scrolling cross axis stretches items to at least the viewport, never below content.
			l.cross = if clips_cross { prim.max(l.cross, inner_cross) } else { inner_cross }
		}
		l.pos = total_cross
		total_cross += l.cross + if i < flines.len - 1 { gap_cross } else { 0 }
	}
	mut cross_size := inner_cross
	if cross_size == unknown {
		own_min := if row { s.min_height.resolve(ah) } else { s.min_width.resolve(aw) }
		own_max := if row { s.max_height.resolve(ah) } else { s.max_width.resolve(aw) }
		frame_cross := if row { f.vertical() } else { f.horizontal() }
		cross_size = prim.max(0,
			clamp_axis(total_cross + frame_cross, own_min, own_max) - frame_cross)
		if flines.len == 1 {
			flines[0].cross = cross_size
		}
	}
	for l in flines {
		for j in l.start .. l.end {
			mut it := unsafe { &items[j] }
			if it.align == .stretch && it.def_cross == unknown && !it.auto_ca && !it.auto_cb {
				it.cross = clamp_axis(l.cross - it.m_cross_a - it.m_cross_b, it.min_cross,
					it.max_cross)
			}
		}
		place_line(mut items, l, space, gap_main, s.justify, s.is_reverse())
	}

	// A container clipping its cross axis reports at most the available size; its stretched
	// children above keep the content size and overflow (that is what it scrolls).
	avail_cross := if row { avail_h } else { avail_w }
	if inner_cross == unknown && clips_cross && avail_cross < unbounded {
		cross_size = prim.min(cross_size, avail_cross)
	}
	content_w := if row { space } else { cross_size }
	content_h := if row { cross_size } else { space }
	out_w := if kw != unknown { kw } else { content_w + f.horizontal() }
	out_h := if kh != unknown { kh } else { content_h + f.vertical() }
	if perform {
		mut extent := prim.Size{}
		for it in items {
			w := if row { it.main } else { it.cross }
			h := if row { it.cross } else { it.main }
			t.layout_node(it.id, w, h, w, h, true)
			px_ := if row { it.pos_main } else { it.pos_cross }
			py_ := if row { it.pos_cross } else { it.pos_main }
			x := f.left + px_
			y := f.top + py_
			t.nodes[it.id].rect = prim.rect(x, y, w, h)
			extent.width = prim.max(extent.width, x + w +
				(if row { it.m_main_b } else { it.m_cross_b }))
			extent.height = prim.max(extent.height, y + h +
				(if row { it.m_cross_b } else { it.m_main_b }))
		}
		t.nodes[id].content = prim.size(extent.width + s.padding.right, extent.height +
			s.padding.bottom)
		t.place_absolutes(absolutes, s, out_w, out_h)
	}
	t.flex_items[depth] = items
	t.flex_lines[depth] = flines
	t.depth--
	return prim.size(out_w, out_h)
}
