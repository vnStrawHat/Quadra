// Table pinned columns, cell selection and copying (kit/data/table_pin.v).
module main

import quadra.core
import quadra.kit
import quadra.kit.data
import quadra.platform
import quadra.testing
import quadra.ui

struct Sheet {
mut:
	table core.View[data.Table]
	cells bool
}

@[markused]
pub fn (mut s Sheet) render(mut cx core.ViewCx[Sheet]) core.Element {
	if s.table.id == 0 {
		mut cols := []data.Column{}
		for i in 0 .. 8 {
			cols << data.Column{
				title:  'C${i}'
				width:  140
				pinned: i == 0
			}
		}
		s.table = data.new_table(mut cx.Cx,
			id:           'sheet'
			columns:      cols
			rows:         100
			select_cells: s.cells
			cell:         fn (row int, col int) string {
				return 'r${row}c${col}'
			}
		)
	}
	return ui.div().size_full().child(cx.child_view(s.table))
}

fn x_of(h &testing.Harness, id string) f32 {
	r := h.find(id) or { panic('${id} missing') }
	return r.x
}

fn sideways(mut h testing.Harness, lines f32) {
	h.inject(platform.WheelEvent{
		dy:   lines
		x:    300
		y:    100
		mods: .shift
	})
	h.frame()
}

fn test_pinned_column_stays_while_the_others_scroll() {
	mut h := testing.new_harness(width: 600, height: 400)!
	kit.install(mut h.app)!
	h.open(Sheet{}, width: 600, height: 400)!
	pinned, other := x_of(h, 'sheet-h0'), x_of(h, 'sheet-h2')
	sideways(mut h, -3) // 120 px to the right
	assert x_of(h, 'sheet-h0') == pinned
	assert x_of(h, 'sheet-h2') == other - 120
	for _ in 0 .. 40 { // far past the end: stops with the last column at the right edge
		sideways(mut h, -3)
	}
	last := h.find('sheet-h7') or { panic('missing') }
	assert last.x + 140 == 600
	sideways(mut h, 100)
	assert x_of(h, 'sheet-h2') == other
}

fn test_cell_cursor_moves_scrolls_and_copies() {
	mut h := testing.new_harness(width: 600, height: 400)!
	kit.install(mut h.app)!
	h.open(Sheet{ cells: true },
		width:  600
		height: 400
	)!
	r := h.find('sheet-row3') or { panic('no row') }
	h.click_at(r.x + 150 + 20, r.y + 5) // row 3, column 1 (the first unpinned)
	h.press('ctrl-c')!
	assert h.plat.clipboard == 'r3c1'
	for _ in 0 .. 6 {
		h.press('right')!
	}
	h.press('ctrl-c')!
	assert h.plat.clipboard == 'r3c7'
	last := h.find('sheet-h7') or { panic('missing') }
	assert last.x + 140 <= 600 // scrolled so the cursor cell shows
	h.press('down')!
	h.press('ctrl-c')!
	assert h.plat.clipboard == 'r4c7'
}

fn test_rows_copy_as_tab_separated_values() {
	mut h := testing.new_harness(width: 600, height: 400)!
	kit.install(mut h.app)!
	h.open(Sheet{}, width: 600, height: 400)!
	h.click('sheet-row1')!
	h.press('shift-down')!
	h.press('ctrl-c')!
	assert h.plat.clipboard == 'r1c0\tr1c1\tr1c2\tr1c3\tr1c4\tr1c5\tr1c6\tr1c7\nr2c0\tr2c1\tr2c2\tr2c3\tr2c4\tr2c5\tr2c6\tr2c7\n'
}
