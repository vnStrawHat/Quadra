// Editor benchmarks (specs/18 budgets; run with -prod): open a 10 MB file, a keystroke plus
// repaint deep in a 100 000-line file, a scroll frame, and closures across typing.
module main

import time
import quadra.core
import quadra.core.life
import quadra.kit
import quadra.kit.code
import quadra.testing
import quadra.ui

struct Host {
mut:
	ed   core.View[code.Editor]
	text string
}

@[markused]
pub fn (mut h Host) render(mut cx core.ViewCx[Host]) core.Element {
	if h.ed.id == 0 {
		h.ed = code.new_editor(mut cx.Cx, id: 'ed', text: h.text, path: 'big.v')
	}
	return ui.div().size_full().child(cx.child_view(h.ed))
}

fn source(lines int) string {
	mut out := []string{cap: lines}
	for i in 0 .. lines {
		out << match i % 5 {
			0 { 'pub fn handler_${i}(mut ctx Context, id int) !string { // entry ${i}' }
			1 { "\tname := 'item-\${id}-${i}' + ctx.prefix.to_upper()" }
			2 { '\tif id > ${i} && ctx.flags & 0x1F != 0 { return error("bad ${i}") }' }
			3 { '\treturn ctx.lookup(name, [1, 2, 3], none) or { "" }' }
			else { '}' }
		}
	}
	return out.join('\n')
}

fn ms(sw time.StopWatch, n int) f64 {
	return f64(sw.elapsed().microseconds()) / f64(n) / 1000
}

fn open(text string) (&testing.Harness, &code.Editor) {
	mut h := testing.new_harness(width: 1400, height: 900) or { panic(err) }
	kit.install(mut h.app) or { panic(err) }
	v := h.open(Host{
		text: text
	},
		width:  1400
		height: 900
	) or { panic(err) }
	mut cx := core.Cx{
		app: h.app
	}
	host := cx.peek(v.model())
	return h, cx.peek(host.ed.model())
}

fn main() {
	// Open: about 10 MB of V source, buffer and first frame.
	big := source(190000)
	osw := time.new_stopwatch()
	mut h0, _ := open(big)
	size := f64(big.len) / 1e6
	println('editor: open ${size:4.1f} MB and paint                ${ms(osw, 1):10.1f} ms    (budget 200)')
	h0.frame()

	// Keystrokes deep in a 100 000-line file (the view follows the cursor).
	mut h, e := open(source(100000))
	h.click('ed') or { panic(err) }
	mut d := e.doc
	at := d.buf.line_end(50000)
	d.set_cursor(at, at, false)
	h.type_text('x') // warm up: reveal, tokenize to the view
	n := 300
	ksw := time.new_stopwatch()
	for _ in 0 .. n {
		h.type_text('x')
	}
	println('editor: keystroke + repaint, line 50 000 of 100 000 ${ms(ksw, n):10.2f} ms    (budget 2)')
	ph := h.window().stats.phase_us
	println('  phases (us): render ${ph[0]}, layout ${ph[1]}, prepaint ${ph[2]}, paint ${ph[3]}')
	// A keystroke near the top invalidates every highlighter state below it.
	d.set_cursor(10, 10, false)
	h.type_text('y')
	tsw := time.new_stopwatch()
	for _ in 0 .. n {
		h.type_text('y')
	}
	println('editor: keystroke + repaint, line 2             ${ms(tsw, n):10.2f} ms    (budget 2)')

	// Scrolling: one wheel step per frame.
	r := h.find('ed') or { panic('no editor') }
	ssw := time.new_stopwatch()
	for _ in 0 .. n {
		h.scroll_at(r.x + 200, r.y + 200, -60)
	}
	println('editor: scroll frame                            ${ms(ssw, n):10.2f} ms')

	// Closures stay flat while typing and blinking.
	before := life.live_closures()
	for _ in 0 .. 500 {
		h.advance(100)
		h.type_text('z')
	}
	println('  closures: ${before} before, ${life.live_closures()} after 500 keystrokes')
}
