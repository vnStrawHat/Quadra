// Memory leak probe: drives the gallery headlessly (hover sweeps, page switches, scrolls)
// and prints the GC heap and the process working set as it goes. A leak shows as a
// number that keeps climbing after the first rounds.
//   bench/leak [fake|real] [rounds] [all|switch|hover|scroll|type|table]   fake: null GPU + fake text; real: WARP + DirectWrite
module main

import os
import quadra.core.life
import apps.gallery.pages
import quadra.native
import quadra.testing

fn main() {
	mode := if os.args.len > 1 { os.args[1] } else { 'fake' }
	rounds := if os.args.len > 2 { os.args[2].int() } else { 20 }
	only := if os.args.len > 3 { os.args[3] } else { 'all' } // all, switch, hover, scroll, type, table
	mut h := testing.new_harness()!
	if mode == 'real' {
		mut ph := testing.new_pixel_harness()!
		h = &ph.Harness
	}
	h.open(pages.Gallery{}, width: 1200, height: 800)!
	ids := pages.all().map(it.id)
	if only == 'type' {
		open_page(mut h, 'text-input')!
	}
	if only == 'table' {
		open_page(mut h, 'table')!
	}
	for r in 0 .. rounds {
		if only == 'type' {
			// 50 keystrokes into the name field, then clear it.
			h.click('in-name')!
			h.type_text('Nguyen Van An dang go phim de do bo nho 0123456789')
			h.press('ctrl-a')!
			h.press('backspace')!
		}
		if only == 'table' {
			// 40 wheel steps down the million-row table, then back up.
			for dy in [f32(-120), 120] {
				for _ in 0 .. 20 {
					h.scroll_at(650, 420, dy)
				}
			}
		}
		for id in ids {
			if only in ['type', 'table'] {
				break
			}
			if only in ['all', 'switch'] {
				open_page(mut h, id) or {}
			}
			for y := f32(40); y < 800; y += 37 {
				for x := f32(260); x < 1200; x += 53 {
					if only in ['all', 'hover'] {
						h.move_to(x, y)
					}
				}
			}
			if only in ['all', 'scroll'] {
				h.scroll_at(700, 400, -300)
				h.scroll_at(700, 400, 300)
			}
			h.advance(600) // timers: caret blink, debounces
		}
		gc_collect()
		u := gc_heap_usage()
		mb := fn (b usize) f64 {
			return f64(b) / (1024 * 1024)
		}
		eprintln(h.app.debug_counts() +
			' uploads ${if h.dev != unsafe { nil } { h.dev.uploads.len } else { -1 }}')
		println('closures ${life.live_closures()}  round ${r:3d}  live ${mb(u.heap_size -
			u.free_bytes - u.unmapped_bytes):8.1f} MB  working set ${f64(native.working_set_bytes()) / (1024 * 1024):8.1f} MB')
	}
}

fn open_page(mut h testing.Harness, id string) ! {
	for k in 0 .. 16 { // down first, then back up
		if h.find('nav-${id}') != none {
			break
		}
		h.scroll_at(100, 400, if k < 8 { -400 } else { 400 })
	}
	h.click('nav-${id}')!
}
