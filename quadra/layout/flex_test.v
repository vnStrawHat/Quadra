module layout

import quadra.prim

// Fake text: 8 px per char, 16 px lines, wraps at spaces-free char boundaries.
struct FakeText {
	chars int
}

fn measure_fake(ctx voidptr, kw f32, kh f32, aw f32, ah f32) prim.Size {
	t := unsafe { &FakeText(ctx) }
	full := f32(t.chars * 8)
	limit := if kw != unknown { kw } else { aw }
	if limit >= unbounded || full <= limit {
		return prim.size(full, 16)
	}
	per_line := int(prim.max(1, limit / 8))
	lines := (t.chars + per_line - 1) / per_line
	return prim.size(f32(per_line * 8), f32(lines * 16))
}

fn row(w f32, h f32) Style {
	return Style{
		width:  px(w)
		height: px(h)
	}
}

fn sized(w f32, h f32) Style {
	return Style{
		width:  px(w)
		height: px(h)
	}
}

fn grow(g f32) Style {
	return Style{
		grow: g
	}
}

fn test_row_fixed_children_stretch_cross() {
	mut t := Tree{}
	a := t.node(Style{ width: px(50) }, [])
	b := t.node(Style{ width: px(70) }, [])
	root := t.node(row(300, 100), [a, b])
	t.compute(root, prim.size(300, 100))
	assert t.rect(a) == prim.rect(0, 0, 50, 100)
	assert t.rect(b) == prim.rect(50, 0, 70, 100)
}

fn test_grow_and_shrink() {
	mut t := Tree{}
	a := t.node(grow(1), [])
	b := t.node(grow(2), [])
	root := t.node(row(300, 50), [a, b])
	t.compute(root, prim.size(300, 50))
	assert t.rect(a).width == 100 && t.rect(b).width == 200 && t.rect(b).x == 100

	mut t2 := Tree{}
	mut ids := []NodeId{}
	for _ in 0 .. 3 {
		ids << t2.node(Style{ basis: px(200) }, [])
	}
	r2 := t2.node(row(300, 50), ids)
	t2.compute(r2, prim.size(300, 50))
	for id in ids {
		assert t2.rect(id).width == 100
	}
}

fn test_max_clamp_freezes_and_redistributes() {
	mut t := Tree{}
	a := t.node(Style{ grow: 1, max_width: px(50) }, [])
	b := t.node(grow(1), [])
	root := t.node(row(300, 10), [a, b])
	t.compute(root, prim.size(300, 10))
	assert t.rect(a).width == 50
	assert t.rect(b).width == 250
}

fn test_justify_variants_and_gap_padding() {
	cases := {
		Justify.start:   [f32(10), 50]
		Justify.end:     [f32(150), 190]
		Justify.center:  [f32(80), 120]
		Justify.between: [f32(10), 190]
		Justify.evenly:  [f32(56.666668), 143.33334]
	}
	for j, want in cases {
		mut t := Tree{}
		a := t.node(sized(40, 10), [])
		b := t.node(sized(40, 10), [])
		root := t.node(Style{
			width:   px(240)
			height:  px(30)
			padding: prim.edges(10)
			justify: j
		}, [a, b])
		t.compute(root, prim.size(240, 30))
		assert t.rect(a).x == want[0], '${j}: a.x ${t.rect(a).x}'
		assert t.rect(b).x == want[1], '${j}: b.x ${t.rect(b).x}'
		assert t.rect(a).y == 10
	}
}

fn test_column_wraps_text_at_stretched_width() {
	mut t := Tree{}
	txt := &FakeText{25} // 200 px unwrapped
	leaf := t.leaf(Style{}, measure_fake, txt)
	other := t.node(Style{ height: px(10) }, [])
	root := t.node(Style{
		direction: .column
		width:     px(100)
		gap_row:   4
	}, [leaf, other])
	t.compute(root, prim.size(100, 400))
	assert t.rect(leaf) == prim.rect(0, 0, 100, 48) // 12 chars per line, 3 lines, stretched to 100
	assert t.rect(other).y == 52
}

fn test_wrap_lines() {
	mut t := Tree{}
	mut ids := []NodeId{}
	for _ in 0 .. 4 {
		ids << t.node(sized(100, 20), [])
	}
	box := t.node(Style{
		width:       px(250)
		wrap:        true
		gap_row:     5
		gap_col:     10
		align_items: .start
	}, ids)
	root := t.node(Style{
		direction:   .column
		align_items: .start
	}, [box])
	t.compute(root, prim.size(250, 500))
	assert t.rect(ids[1]) == prim.rect(110, 0, 100, 20)
	assert t.rect(ids[2]) == prim.rect(0, 25, 100, 20)
	assert t.rect(box).height == 45
}

fn test_absolute_and_auto_margin() {
	mut t := Tree{}
	abs := t.node(Style{
		position:    .absolute
		inset_right: px(10)
		inset_top:   px(10)
		width:       px(20)
		height:      px(20)
	}, [])
	pushed := t.node(Style{
		width:       px(30)
		margin_auto: .left
	}, [])
	root := t.node(row(200, 100), [abs, pushed])
	t.compute(root, prim.size(200, 100))
	assert t.rect(abs) == prim.rect(170, 10, 20, 20)
	assert t.rect(pushed).x == 170
}

fn test_auto_sized_container_fits_content() {
	mut t := Tree{}
	a := t.node(sized(40, 10), [])
	b := t.node(sized(60, 30), [])
	inner := t.node(Style{ gap_col: 5 }, [a, b])
	root := t.node(Style{
		direction:   .column
		width:       px(500)
		height:      px(500)
		align_items: .start
	}, [inner])
	t.compute(root, prim.size(500, 500))
	assert t.rect(inner) == prim.rect(0, 0, 105, 30)
	assert t.rect(b).x == 45
}

fn test_measure_cache_limits_work() {
	mut t := Tree{}
	mut parent := t.node(Style{ direction: .column }, [])
	root := parent
	for _ in 0 .. 30 {
		child := t.node(Style{ direction: .column }, [])
		t.add_child(parent, child)
		parent = child
	}
	txt := &FakeText{10}
	t.add_child(parent, t.leaf(Style{}, measure_fake, txt))
	t.compute(root, prim.size(300, 300))
	// Without caching, nested auto columns re-measure exponentially.
	assert t.measure_calls < 200, '${t.measure_calls} evaluations'
}

fn test_auto_height_column_in_scroll_keeps_content_height() {
	// A page column taller than its scroll viewport: its children must not shrink.
	mut t := Tree{}
	a := t.node(sized(100, 300), [])
	b := t.node(sized(100, 300), [])
	page := t.node(Style{
		direction: .column
		gap_row:   10
	}, [a, b])
	root := t.node(Style{
		direction:  .column
		width:      px(200)
		height:     px(400)
		overflow_y: .scroll
	}, [page])
	t.compute(root, prim.size(200, 400))
	assert t.rect(page).height == 610
	assert t.rect(b) == prim.rect(0, 310, 100, 300)
}

fn test_row_scrolling_y_reports_viewport_height() {
	// header (fixed) + body (grow) where body is a row that scrolls vertically.
	mut t := Tree{}
	tall := t.node(sized(100, 2000), [])
	body := t.node(Style{
		grow:       1
		overflow_y: .scroll
	}, [tall])
	header := t.node(Style{ height: px(52) }, [])
	root := t.node(Style{
		direction: .column
		width:     px(300)
		height:    px(800)
	}, [header, body])
	t.compute(root, prim.size(300, 800))
	assert t.rect(header).height == 52
	assert t.rect(body) == prim.rect(0, 52, 300, 748)
	assert t.rect(tall).height == 2000
}

fn test_row_scrolling_y_stretches_child_to_content() {
	// The gallery body: a row scrolling vertically holds a tall page column.
	mut t := Tree{}
	a := t.node(sized(100, 500), [])
	b := t.node(sized(100, 500), [])
	page := t.node(Style{ direction: .column }, [a, b])
	body := t.node(Style{
		width:      px(300)
		height:     px(400)
		overflow_y: .scroll
	}, [page])
	t.compute(body, prim.size(300, 400))
	assert t.rect(page).height == 1000
	assert t.rect(b).height == 500
	assert t.content_size(body).height == 1000
}

fn test_sole_grower_is_not_measured_for_its_base() {
	// header + growing body in a fixed column: the body takes the rest whatever its content.
	mut t := Tree{}
	txt := &FakeText{2000}
	leaf := t.leaf(Style{}, measure_fake, voidptr(txt))
	body := t.node(Style{
		grow:      1
		direction: .column
	}, [leaf])
	header := t.node(Style{ height: px(52) }, [])
	root := t.node(Style{
		direction: .column
		width:     px(300)
		height:    px(800)
	}, [header, body])
	t.compute(root, prim.size(300, 800))
	assert t.rect(header).height == 52
	assert t.rect(body) == prim.rect(0, 52, 300, 748)
	// root, header (perform), body (perform), leaf (perform + its base inside the body)
	assert t.measure_calls <= 6, '${t.measure_calls} evaluations'
}
