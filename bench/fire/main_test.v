module main

import quadra.kit.term as vt

fn test_a_frame_fills_the_screen_without_scrolling() {
	mut f := new_fire(40, 10)
	for _ in 0 .. 50 {
		f.step()
	}
	f.frame()
	mut s := vt.new_screen(40, 10)
	mut p := vt.Parser{}
	p.feed(mut s, f.out)
	assert s.scrollback.len() == 0 && s.y == 9 && s.x == 39
	for row in s.lines {
		assert row.cells.all(it.ch == u32(`▀`))
	}
	assert s.lines[9].cells[3].bg == vt.palette(int(palette.last())) // the burning row
	assert s.lines[0].cells.any(it.fg != s.lines[9].cells[3].bg) // cooler at the top
}
