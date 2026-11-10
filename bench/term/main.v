// Terminal benchmarks (specs/16 budgets; run with -prod): parse throughput, a 200x60
// repaint, and the scrollback cost of 100 000 lines.
module main

import time
import quadra.core
import quadra.core.life
import quadra.kit
import quadra.kit.term
import quadra.platform
import quadra.testing
import quadra.ui

fn mb_per_s(name string, data []u8, rounds int) {
	mut s := term.new_screen(200, 60)
	mut p := term.Parser{}
	p.feed(mut s, data) // warm up
	sw := time.new_stopwatch()
	for _ in 0 .. rounds {
		p.feed(mut s, data)
	}
	secs := f64(sw.elapsed().microseconds()) / 1e6
	mb := f64(data.len) * rounds / (1024 * 1024)
	println('${name:-44s} ${mb / secs:10.1f} MB/s  (budget 50)')
}

// fire_frames is what bench/fire sends: per cell a 256-color foreground and background when
// they change, then a half block; the frame starts at the top left.
fn fire_frames(cols int, rows int, n int) []u8 {
	mut out := []u8{}
	mut seed := u32(0x9e3779b9)
	for _ in 0 .. n {
		out << '\x1b[H'.bytes()
		for _ in 0 .. cols * rows {
			seed ^= seed << 13
			seed ^= seed >> 17
			seed ^= seed << 5
			if seed & 3 != 0 {
				out << '\x1b[38;5;${16 + (seed >> 8) % 216}m'.bytes()
			}
			if seed & 12 != 0 {
				out << '\x1b[48;5;${16 + (seed >> 16) % 216}m'.bytes()
			}
			out << '▀'.bytes()
		}
	}
	return out
}

fn heap_mb() f64 {
	gc_collect()
	u := gc_heap_usage()
	return f64(u.heap_size - u.free_bytes - u.unmapped_bytes) / (1024 * 1024)
}

struct Host {
mut:
	term core.View[term.Terminal]
	fake &term.FakePty = unsafe { nil }
}

@[markused]
pub fn (mut h Host) render(mut cx core.ViewCx[Host]) core.Element {
	if h.term.id == 0 {
		h.term = term.new_terminal(mut cx.Cx, id: 't')
		h.fake = &term.FakePty{}
		term.start(mut cx.Cx, h.term, h.fake.spawner(), platform.PtyOptions{}) or { panic(err) }
	}
	return ui.div().size_full().child(cx.child_view(h.term))
}

fn main() {
	mut plain := []u8{}
	mut sgr := []u8{}
	for i in 0 .. 20000 {
		plain << 'drwxr-xr-x  2 user staff  4096 Nov  7 12:00 some-directory-name-${i}\r\n'.bytes()
		sgr << '\x1b[1;34mdir-${i}\x1b[0m  \x1b[32mrun.sh\x1b[0m  \x1b[38;5;208mlog\x1b[0m  \x1b[48;2;10;20;30m ok \x1b[0m\r\n'.bytes()
	}
	mb_per_s('term: parse plain text (1.3 MB)', plain, 20)
	mb_per_s('term: parse SGR-heavy output', sgr, 20)
	fire := fire_frames(200, 60, 20)
	mb_per_s('term: parse 256-color half blocks (fire)', fire, 20)

	// Scrollback: 100 000 shell-like lines.
	before := heap_mb()
	mut s := term.new_screen(80, 24)
	s.scrollback.max = 100000
	mut p := term.Parser{}
	for i in 0 .. 100000 {
		p.feed(mut s, 'total ${i}: -rw-r--r-- 1 user staff 1234 file-${i}.txt\r\n'.bytes())
	}
	println('term: 100 000 lines of scrollback              ${heap_mb() - before:10.1f} MB    (budget 40)')

	// Repaint: a full 200x60 screen of colored text.
	mut h := testing.new_harness(width: 1800, height: 1100) or { panic(err) }
	kit.install(mut h.app) or { panic(err) }
	v := h.open(Host{}, width: 1800, height: 1100) or { panic(err) }
	mut cx := core.Cx{
		app: h.app
	}
	host := cx.peek(v.model())
	t := cx.peek(host.term.model())
	mut fake := host.fake
	fake.output(sgr[..60 * 90].bytestr())
	h.frame()
	println('  screen ${t.screen.cols}x${t.screen.rows}')
	mut w := h.window()
	n := 100
	w.invalidate()
	w.draw_frame()
	sw := time.new_stopwatch()
	for _ in 0 .. n {
		w.invalidate()
		w.draw_frame()
	}
	per := f64(sw.elapsed().microseconds()) / f64(n) / 1000
	println('term: repaint the full screen                  ${per:10.2f} ms    (budget 2)')
	ph := w.stats.phase_us
	println('  phases (us): render ${ph[0]}, layout ${ph[1]}, prepaint ${ph[2]}, paint ${ph[3]}')

	// A fire frame through the inbox and the window, as OneTerm takes each one under load.
	frame := fire_frames(t.screen.cols, t.screen.rows, 1).bytestr()
	fake.output(frame)
	h.frame()
	st0 := t.stats
	fsw := time.new_stopwatch()
	for _ in 0 .. n {
		fake.output(frame)
		h.frame()
	}
	fire_ms := f64(fsw.elapsed().microseconds()) / f64(n) / 1000
	parse_ms := f64(t.stats.parse_us - st0.parse_us) / f64(n) / 1000
	paint_ms := f64(t.stats.paint_us - st0.paint_us) / f64(n) / 1000
	println('term: fire frame (parse, render, paint)        ${fire_ms:10.2f} ms')
	fpb := w.stats.phase_bytes
	println('  parse ${parse_ms:.2f} ms, grid paint ${paint_ms:.2f} ms, ${(fpb[0] + fpb[1] +
		fpb[2] + fpb[3]) / 1024} KB allocated by render to paint')

	// Streaming: closures and heap must stay flat while output flows through the inbox.
	for round in 0 .. 3 {
		for i in 0 .. 1000 {
			fake.output('line ${i} of streamed output\r\n')
			h.frame()
		}
		println('  stream round ${round}: closures ${life.live_closures()}, heap ${heap_mb():6.1f} MB')
	}

	// Resize: 10 000 lines of history reflowed on every width change.
	mut rs := term.new_screen(100, 24)
	mut rp := term.Parser{}
	for i in 0 .. 10000 {
		rp.feed(mut rs, 'total ${i}: -rw-r--r-- 1 user staff 1234 file-${i}.txt\r\n'.bytes())
	}
	rsw := time.new_stopwatch()
	for i in 0 .. 50 {
		rs.resize(if i % 2 == 0 { 120 } else { 100 }, 24)
	}
	println('term: resize with 10 000 history lines          ${f64(rsw.elapsed().microseconds()) / 50000:10.2f} ms')
}
