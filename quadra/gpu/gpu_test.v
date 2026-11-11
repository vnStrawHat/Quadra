module gpu

import quadra.prim

fn sprite(page int, x f32, y f32) Sprite {
	return Sprite{
		bounds: prim.rect(x, y, 10, 10)
		tile:   AtlasTile{
			page: page
		}
	}
}

fn test_instance_is_144_bytes() {
	assert sizeof(Instance) == 144
}

fn test_single_page_scene_is_one_batch() {
	mut s := Scene{}
	for i in 0 .. 100 {
		s.quad(Quad{
			bounds:     prim.rect(f32(i * 20), 0, 18, 18)
			background: prim.rgb(0x2563EB)
		}, ContentMask{})
		s.mono(sprite(0, f32(i * 20), 2), ContentMask{})
	}
	b := s.batches()
	assert b.len == 1 && b[0].count == 200
}

fn test_page_switches_split_and_optimize_regroups() {
	mut s := Scene{}
	// Ten separate buttons, labels alternating between two atlas pages.
	for i in 0 .. 10 {
		x := f32(i * 100)
		s.quad(Quad{
			bounds:     prim.rect(x, 0, 90, 30)
			background: prim.rgb(0xFFFFFF)
		}, ContentMask{})
		s.mono(sprite(i % 2, x + 5, 5), ContentMask{})
	}
	assert s.batches().len > 2
	mut h := SpatialHash{}
	s.optimize(mut h)
	assert s.batches().len == 2 // all quads + page 0 labels, then page 1 labels
	// Every label is still drawn after the quad under it.
	for i, inst in s.instances {
		if inst.kind != u32(Kind.mono) {
			continue
		}
		for j in i + 1 .. s.instances.len {
			other := s.instances[j]
			assert !(other.kind == u32(Kind.quad) && other.rect().intersects(inst.rect()))
		}
	}
}

fn test_overlapping_order_is_preserved() {
	mut s := Scene{}
	s.mono(sprite(1, 0, 0), ContentMask{})
	s.quad(Quad{
		bounds:     prim.rect(5, 5, 10, 10)
		background: prim.rgb(0)
	}, ContentMask{}) // covers the sprite
	s.mono(sprite(0, 100, 100), ContentMask{})
	mut h := SpatialHash{}
	s.optimize(mut h)
	mut quad_i, mut covered_i := -1, -1
	for i, inst in s.instances {
		if inst.kind == u32(Kind.quad) {
			quad_i = i
		}
		if inst.kind == u32(Kind.mono) && inst.bounds[0] == 0 {
			covered_i = i
		}
	}
	assert covered_i < quad_i
}

fn test_atlas_packs_and_adds_pages() {
	mut a := new_atlas(.mono, 64)
	mut tiles := []AtlasTile{}
	for _ in 0 .. 20 {
		tiles << a.insert(15, 15, []u8{len: 225})!
	}
	assert a.page_count() == 2 // 4 x 4 tiles of 16 px fit on a 64 px page
	assert tiles[0].page == 0 && tiles[19].page == 1
	for i, t in tiles {
		for j in i + 1 .. tiles.len {
			u := tiles[j]
			same_page := t.page == u.page
			overlap := t.x < u.x + u.w && u.x < t.x + t.w && t.y < u.y + u.h && u.y < t.y + t.h
			assert !(same_page && overlap)
		}
	}
	assert a.take_uploads().len == 20
	assert a.take_uploads().len == 0
	if _ := a.insert(100, 10, []u8{}) {
		assert false, 'oversized tile must fail'
	}
}

fn test_released_tiles_open_their_shelf_again() {
	mut a := new_atlas(.mono, 64)
	one := a.insert(15, 15, []u8{len: 225})!
	two := a.insert(15, 15, []u8{len: 225})!
	high := a.insert(15, 31, []u8{len: 465})!
	a.release(one)
	third := a.insert(15, 15, []u8{len: 225})! // the shelf still has a live tile: no reuse
	assert third.x == two.x + 16 && third.y == one.y
	a.release(two)
	a.release(third)
	again := a.insert(10, 10, []u8{len: 100})! // an emptied shelf takes any bitmap that fits
	assert again.x == 0 && again.y == one.y
	a.release(high)
	assert a.pages[0].next_y == high.y // the last shelf is gone
	a.release(again)
	assert a.pages[0].shelves.len == 0 && a.pages[0].next_y == 0
	for _ in 0 .. 100 { // churn stays on one page
		t := a.insert(40, 40, []u8{len: 1600})!
		a.release(t)
	}
	assert a.page_count() == 1
}

fn test_the_first_atlas_page_is_small() {
	mut a := new_atlas(.poly, 2048)
	small := a.insert(10, 10, []u8{})!
	assert small.page == 0 && small.u1 == f32(10) / 1024
	big := a.insert(1500, 20, []u8{})! // wider than the first page: a full-size one
	assert big.page == 1 && big.u1 == f32(1500) / 2048
	assert page_extent(2048, 0) == 1024 && page_extent(2048, 1) == 2048
	assert page_extent(64, 0) == 64
}

fn test_a_gradient_keeps_its_end_color_under_a_border() {
	mut s := Scene{}
	s.quad(Quad{
		bounds:        prim.rect(0, 0, 100, 34)
		background:    prim.rgb(0x202429)
		gradient_to:   prim.rgb(0x1E2227)
		border_widths: prim.Edges{
			bottom: 1
		}
		border_color:  prim.rgb(0x3E4451)
	}, ContentMask{})
	assert s.instances.len == 2
	assert s.instances[0].flags == 1 && s.instances[0].color2 == prim.rgb(0x1E2227).to_array()
	assert s.instances[1].flags == 2 && s.instances[1].color2 == prim.rgb(0x3E4451).to_array()
}
