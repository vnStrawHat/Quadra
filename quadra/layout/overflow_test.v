// Overflow only clips (specs/05): a hidden container keeps its content size.
module layout

import quadra.prim

// measure_text is 320 px of text in 16 px lines, wrapped at the width it gets.
fn measure_text(_ prim.Holder, kw f32, _ f32, aw f32, _ f32) prim.Size {
	limit := if kw != unknown { kw } else { aw }
	if limit >= unbounded || limit >= 320 {
		return prim.size(320, 16)
	}
	lines := int(320 / limit + 0.999)
	return prim.size(limit, f32(lines * 16))
}

fn test_hidden_overflow_clips_but_keeps_content_height() {
	// A rounded table (overflow hidden, for its corners) inside a page column that scrolls:
	// the table is as tall as its rows, whatever the viewport, so nothing above or below it
	// is squeezed (specs/05: overflow only clips).
	mut t := Tree{}
	mut rows := []NodeId{}
	for _ in 0 .. 10 {
		rows << t.node(Style{
			width:  px(100)
			height: px(50)
		}, [])
	}
	table := t.node(Style{
		direction:  .column
		overflow_x: .hidden
		overflow_y: .hidden
	}, rows)
	para := t.leaf(Style{}, measure_text, prim.Holder{})
	page := t.node(Style{
		direction: .column
	}, [para, table])
	root := t.node(Style{
		direction:  .column
		width:      px(200)
		height:     px(300)
		overflow_y: .scroll
	}, [page])
	t.compute(root, prim.size(200, 300))
	assert t.rect(table).height == 500
	assert t.rect(para).height == 32 // 320 px of text wraps to two lines at 200
	assert t.rect(rows[9]) == prim.rect(0, 450, 100, 50)
}
