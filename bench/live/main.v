// Live data benchmarks (specs/19 budgets; run with -prod): a million log lines held, the
// frame time while 10 000 lines per second stream in, filters over the whole store, and a
// keyed update of a 10 000-row table.
module main

import time
import quadra.core
import quadra.core.life
import quadra.kit
import quadra.kit.data
import quadra.testing
import quadra.ui

fn heap_mb() f64 {
	gc_collect()
	u := gc_heap_usage()
	return f64(u.heap_size - u.free_bytes - u.unmapped_bytes) / (1024 * 1024)
}

fn line(i int) string {
	return match i % 9 {
		4 { '\x1b[33mWARN \x1b[0m cache miss for key user:${i} in region eu-west-1' }
		7 { '\x1b[31mERROR\x1b[0m upstream timed out after 3000ms (attempt ${i % 3 + 1})' }
		else { '\x1b[32mINFO \x1b[0m GET /api/v1/items/${i * 7 % 500} 200 ${i * 13 % 90}ms' }
	}
}

fn batch(from int, n int) []data.LogEntry {
	return []data.LogEntry{len: n, init: data.LogEntry{i64(1793966400000) + i64(from + index), line(
		from + index)}}
}

struct Host {
mut:
	log   core.View[data.LogView]
	table core.View[data.Table]
	rows  []int
	cell  data.CellFn = unsafe { nil }
}

@[markused]
pub fn (mut h Host) render(mut cx core.ViewCx[Host]) core.Element {
	if h.log.id == 0 {
		h.log = data.new_log_view(mut cx.Cx, id: 'log')
		h.table = data.new_table(mut cx.Cx,
			id:      't'
			columns: [data.Column{
				title: 'Key'
			}, data.Column{
				title:   'Value'
				numeric: true
			}]
			cell:    fn (_ int, _ int) string {
				return ''
			}
		)
	}
	return ui.div().size_full().flex_col().child(ui.div().h(600).child(cx.child_view(h.log))).child(ui.div().h(300).child(cx.child_view(h.table)))
}

fn ms(sw time.StopWatch, n int) f64 {
	return f64(sw.elapsed().microseconds()) / f64(n) / 1000
}

fn main() {
	mut h := testing.new_harness(width: 1400, height: 900) or { panic(err) }
	kit.install(mut h.app) or { panic(err) }
	v := h.open(Host{}, width: 1400, height: 900) or { panic(err) }
	mut cx := core.Cx{
		app: h.app
	}
	mut host := cx.peek(v.model())
	lv := cx.peek(host.log.model())

	// A million lines.
	before := heap_mb()
	osw := time.new_stopwatch()
	for k in 0 .. 100 {
		data.append_logs(mut cx, host.log, batch(k * 10000, 10000))
	}
	println('log: append 1 000 000 lines                    ${ms(osw, 1):10.1f} ms')
	println('log: 1 000 000 lines held                      ${heap_mb() - before:10.1f} MB')
	h.frame()

	// Streaming: 10 000 lines per second is 167 lines per 60 Hz frame.
	n := 120
	ssw := time.new_stopwatch()
	for k in 0 .. n {
		data.append_logs(mut cx, host.log, batch(1000000 + k * 167, 167))
		h.frame()
	}
	println('log: frame with 167 new lines (10 000/s)        ${ms(ssw, n):10.2f} ms    (budget 4)')
	ph := h.window().stats.phase_us
	println('  phases (us): render ${ph[0]}, layout ${ph[1]}, prepaint ${ph[2]}, paint ${ph[3]}')

	// Filters over the whole store.
	mut lvm := cx.peek(host.log.model())
	fsw := time.new_stopwatch()
	lvm.set_filter('timed out', false)
	println('log: text filter over ${lv.store.len()} lines        ${ms(fsw, 1):10.1f} ms  (${lv.shown_count()} match)')
	rsw := time.new_stopwatch()
	lvm.set_filter(r'attempt [23]', true)
	println('log: regex filter over ${lv.store.len()} lines       ${ms(rsw, 1):10.1f} ms  (${lv.shown_count()} match)')
	fl := time.new_stopwatch()
	for k in 0 .. n {
		data.append_logs(mut cx, host.log, batch(2000000 + k * 167, 167))
		h.frame()
	}
	println('log: frame with 167 new lines, regex filter    ${ms(fl, n):10.2f} ms    (budget 4)')

	// A keyed update of 10 000 rows (all compared for flashes).
	host.rows = []int{len: 10000, init: index}
	holder := host
	host.cell = fn [holder] (row int, col int) string {
		return if col == 0 { 'key-${row}' } else { holder.rows[row].str() }
	}
	keys := []string{len: 10000, init: 'key-${index}'}
	data.set_keyed_rows(mut cx, host.table, keys, host.cell)
	h.frame()
	tsw := time.new_stopwatch()
	for k in 0 .. 20 {
		for i in 0 .. 100 {
			host.rows[(k * 100 + i * 37) % 10000] += 1
		}
		data.set_keyed_rows(mut cx, host.table, keys, host.cell)
		h.frame()
	}
	println('table: keyed update of 10 000 rows + frame     ${ms(tsw, 20):10.2f} ms')
	before_c := life.live_closures()
	for k in 0 .. 200 {
		data.append_logs(mut cx, host.log, batch(3000000 + k * 10, 10))
		data.set_keyed_rows(mut cx, host.table, keys, host.cell)
		h.frame()
	}
	println('  closures: ${before_c} before, ${life.live_closures()} after 200 updates')
}
