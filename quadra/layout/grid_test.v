module layout

import quadra.prim

fn grid_root(cols []Track, w f32) Style {
	return Style{
		display: .grid
		width:   px(w)
		grid:    &GridStyle{
			columns: cols
		}
	}
}

fn test_grid_fixed_and_fr_columns() {
	mut t := Tree{}
	mut ids := []NodeId{}
	for _ in 0 .. 6 {
		ids << t.node(Style{ height: px(20) }, [])
	}
	mut s := grid_root([track_px(100), fr(1), fr(2)], 400)
	s.gap_row = 10
	root := t.node(s, ids)
	t.compute(root, prim.size(400, 1000))
	assert t.rect(ids[0]) == prim.rect(0, 0, 100, 20)
	assert t.rect(ids[1]) == prim.rect(100, 0, 100, 20)
	assert t.rect(ids[2]) == prim.rect(200, 0, 200, 20)
	assert t.rect(ids[3]) == prim.rect(0, 30, 100, 20) // second row after the gap
	assert t.content_size(root).height == 50
}

fn test_grid_auto_rows_follow_tallest_item() {
	mut t := Tree{}
	a := t.node(Style{}, []) // auto height: stretched to the row
	b := t.node(Style{ height: px(40) }, [])
	root := t.node(grid_root([fr(1), fr(1)], 200), [a, b])
	t.compute(root, prim.size(200, 500))
	assert t.rect(a).height == 40
	assert t.rect(b).height == 40
}

fn test_grid_areas_and_spans() {
	mut t := Tree{}
	header := t.node(Style{
		grid: &GridStyle{
			area: 'header'
		}
	}, [])
	nav := t.node(Style{
		grid: &GridStyle{
			area: 'nav'
		}
	}, [])
	main := t.node(Style{
		grid: &GridStyle{
			area: 'main'
		}
	}, [])
	root := t.node(Style{
		display: .grid
		width:   px(300)
		height:  px(200)
		grid:    &GridStyle{
			columns: [track_px(100), fr(1)]
			rows:    [track_px(50), fr(1)]
			areas:   ['header header', 'nav main']
		}
	}, [main, nav, header])
	t.compute(root, prim.size(300, 200))
	assert t.rect(header) == prim.rect(0, 0, 300, 50)
	assert t.rect(nav) == prim.rect(0, 50, 100, 150)
	assert t.rect(main) == prim.rect(100, 50, 200, 150)
}

fn test_grid_auto_fill_responsive_columns() {
	mut t := Tree{}
	mut ids := []NodeId{}
	for _ in 0 .. 4 {
		ids << t.node(Style{ height: px(10) }, [])
	}
	root := t.node(Style{
		display: .grid
		width:   px(500)
		gap_col: 10
		grid:    &GridStyle{
			fill_min: 150
		}
	}, ids)
	t.compute(root, prim.size(500, 500))
	assert t.rect(ids[0]).width == 160 // 3 columns of (500 - 20) / 3
	assert t.rect(ids[2]).x == 340
	assert t.rect(ids[3]).y == 10 // wrapped to the next row
}

fn test_grid_explicit_placement_and_auto_flow() {
	mut t := Tree{}
	pinned := t.node(Style{
		grid: &GridStyle{
			col_start: 2
			row_start: 1
		}
	}, [])
	a := t.node(Style{ height: px(10) }, [])
	b := t.node(Style{ height: px(10) }, [])
	root := t.node(grid_root([track_px(10), track_px(10)], 20), [pinned, a, b])
	t.compute(root, prim.size(20, 100))
	assert t.rect(pinned).x == 10 && t.rect(pinned).y == 0
	assert t.rect(a).x == 0 && t.rect(a).y == 0 // fills the free cell before it
	assert t.rect(b) == prim.rect(0, 10, 10, 10) // next row: both cells of row 1 are taken
}
