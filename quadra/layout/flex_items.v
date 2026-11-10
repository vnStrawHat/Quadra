// Flex item construction (base sizes) and line collection.
module layout

import quadra.prim

fn (mut t Tree) flex_item(c NodeId, cs &Style, ax &Axes, align_items Align, sole_grower bool) FlexItem {
	row := ax.row
	m := cs.margin
	mut cw := cs.width.resolve(ax.inner_w)
	mut ch := cs.height.resolve(ax.inner_h)
	if cs.aspect_ratio > 0 {
		if cw != unknown && ch == unknown {
			ch = cw / cs.aspect_ratio
		} else if ch != unknown && cw == unknown {
			cw = ch * cs.aspect_ratio
		}
	}
	inner_main := if row { ax.inner_w } else { ax.inner_h }
	inner_cross := if row { ax.inner_h } else { ax.inner_w }
	mut it := FlexItem{
		id:        c
		def_cross: if row { ch } else { cw }
		min_main:  zero_if_unknown((if row { cs.min_width } else { cs.min_height }).resolve(inner_main))
		max_main:  (if row { cs.max_width } else { cs.max_height }).resolve(inner_main)
		min_cross: zero_if_unknown((if row { cs.min_height } else { cs.min_width }).resolve(inner_cross))
		max_cross: (if row { cs.max_height } else { cs.max_width }).resolve(inner_cross)
		m_main_a:  if row { m.left } else { m.top }
		m_main_b:  if row { m.right } else { m.bottom }
		m_cross_a: if row { m.top } else { m.left }
		m_cross_b: if row { m.bottom } else { m.right }
		auto_ma:   cs.margin_auto.has(if row { AutoMargin.left } else { AutoMargin.top })
		auto_mb:   cs.margin_auto.has(if row { AutoMargin.right } else { AutoMargin.bottom })
		auto_ca:   cs.margin_auto.has(if row { AutoMargin.top } else { AutoMargin.left })
		auto_cb:   cs.margin_auto.has(if row { AutoMargin.bottom } else { AutoMargin.right })
		grow:      cs.grow
		shrink:    cs.shrink
		align:     match cs.align_self {
			.auto { align_items }
			.stretch { Align.stretch }
			.start { Align.start }
			.end { Align.end }
			.center { Align.center }
		}
	}
	// Clamped now, not only at the end: the item's content is measured at this size, so text
	// must wrap at the width it ends up with.
	if it.def_cross != unknown {
		it.def_cross = clamp_axis(it.def_cross, it.min_cross, it.max_cross)
	}
	def_main := if row { cw } else { ch }
	basis := cs.basis.resolve(inner_main)
	if basis != unknown {
		it.base = basis
	} else if def_main != unknown {
		it.base = def_main
	} else if sole_grower && cs.shrink > 0 && it.min_main == 0 && it.max_main == unknown {
		it.base = 0 // takes the leftover space: no need to measure its content
	} else {
		// Content size. Items that will stretch are measured at the stretched cross size,
		// so wrapping text in a column gets the right height.
		mut cross_known := it.def_cross
		if cross_known == unknown && it.align == .stretch && ax.stretch_into != unknown {
			cross_known = clamp_axis(prim.max(0, ax.stretch_into - it.m_cross_a - it.m_cross_b),
				it.min_cross, it.max_cross)
		}
		avail_main := inner_of(if row { ax.avail_w } else { ax.avail_h }, it.m_main_a + it.m_main_b)
		avail_cross := inner_of(if row { ax.avail_h } else { ax.avail_w }, it.m_cross_a +
			it.m_cross_b)
		kw2 := if row { unknown } else { cross_known }
		kh2 := if row { cross_known } else { unknown }
		sz := t.layout_node(c, kw2, kh2, if row { avail_main } else { avail_cross }, if row {
			avail_cross
		} else {
			avail_main
		}, false)
		it.base = if row { sz.width } else { sz.height }
	}
	it.hyp = prim.max(0, clamp_axis(it.base, it.min_main, it.max_main))
	it.main = it.hyp
	return it
}

fn zero_if_unknown(v f32) f32 {
	return if v == unknown { 0 } else { v }
}

// collect_lines fills lines (cleared by the caller) with the flex lines of items.
fn collect_lines(items []FlexItem, wrap bool, limit f32, gap f32, mut lines []FlexLine) {
	if !wrap || limit >= unbounded || items.len == 0 {
		lines << FlexLine{
			start: 0
			end:   items.len
		}
		return
	}
	mut start := 0
	mut used := f32(0)
	for i, it in items {
		outer := it.hyp + it.m_main_a + it.m_main_b
		if i > start && used + gap + outer > limit {
			lines << FlexLine{
				start: start
				end:   i
			}
			start = i
			used = outer
		} else {
			used += if i > start { gap + outer } else { outer }
		}
	}
	lines << FlexLine{
		start: start
		end:   items.len
	}
}

fn line_outer(items []FlexItem, l FlexLine, gap f32, hyp bool) f32 {
	mut sum := f32(0)
	for j in l.start .. l.end {
		sum += (if hyp { items[j].hyp } else { items[j].main }) + items[j].m_main_a +
			items[j].m_main_b
	}
	return sum + gap * f32(prim.max(0, l.end - l.start - 1))
}
