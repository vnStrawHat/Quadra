// Terminal load test: a flame drawn in half-block cells, as many frames as the terminal takes.
// Run it inside the terminal under test; it prints frames per second and throughput at the end.
//   tools/v.sh -cc clang -prod -o .tmp/fire.exe bench/fire
//   .tmp/fire.exe [--seconds 15] [--size 200x60] [--report result.txt]
// --report also writes the result line to a file, for scripted runs (bench/fire/run.sh).
module main

import os
import term
import time

fn C.SetConsoleOutputCP(cp u32) bool
fn C.EnumDisplaySettingsW(name voidptr, mode u32, dm voidptr) bool

// display_hz is the refresh rate of the primary display (DEVMODEW.dmDisplayFrequency): vsync
// may cap what a terminal shows, so a report names it.
fn display_hz() int {
	mut dm := [220]u8{}
	dm[68] = 220 // dmSize
	if !C.EnumDisplaySettingsW(unsafe { nil }, 0xffffffff, &dm[0]) {
		return 0
	}
	return int(u32(dm[184]) | u32(dm[185]) << 8)
}

// xterm 256-color indices, coldest first.
const palette = [u8(16), 232, 233, 52, 88, 124, 160, 196, 202, 208, 214, 220, 226, 227, 228, 229,
	230, 231]

struct Fire {
	w int
	h int // two pixels per terminal row
mut:
	heat []u8
	seed u32 = 0x9e3779b9
	out  []u8
	fg   []string
	bg   []string
}

fn new_fire(cols int, rows int) Fire {
	w, h := cols, rows * 2
	mut f := Fire{
		w:    w
		h:    h
		heat: []u8{len: w * h}
		out:  []u8{cap: cols * rows * 24}
		fg:   []string{len: 256, init: '\x1b[38;5;${index}m'}
		bg:   []string{len: 256, init: '\x1b[48;5;${index}m'}
	}
	for x in 0 .. w {
		f.heat[(h - 1) * w + x] = u8(palette.len - 1) // the bottom row burns forever
	}
	return f
}

@[inline]
fn (mut f Fire) next() u32 {
	mut x := f.seed
	x ^= x << 13
	x ^= x >> 17
	x ^= x << 5
	f.seed = x
	return x
}

// step carries every pixel up one row, drifting sideways. Cooling is scaled to the height, so
// the flame fills the screen at any size.
fn (mut f Fire) step() {
	for y in 1 .. f.h {
		for x in 0 .. f.w {
			r := f.next()
			mut to := x + int(r & 3) - 1
			if to < 0 || to >= f.w {
				to = x
			}
			v := f.heat[y * f.w + x]
			cool := v > 0 && (r >> 2) % u32(f.h) < u32(palette.len)
			f.heat[(y - 1) * f.w + to] = if cool { v - 1 } else { v }
		}
	}
}

@[inline]
fn (mut f Fire) put(s string) {
	unsafe { f.out.push_many(s.str, s.len) }
}

// frame builds one screen. A color is sent only when it changes, as a real program would.
fn (mut f Fire) frame() {
	f.out.clear()
	f.put('\x1b[H')
	mut last_fg, mut last_bg := -1, -1
	for row in 0 .. f.h / 2 {
		for x in 0 .. f.w {
			top := int(palette[f.heat[row * 2 * f.w + x]])
			low := int(palette[f.heat[(row * 2 + 1) * f.w + x]])
			if top != last_fg {
				f.put(f.fg[top])
				last_fg = top
			}
			if low != last_bg {
				f.put(f.bg[low])
				last_bg = low
			}
			f.put('▀')
		}
	}
}

fn arg(name string, def string) string {
	i := os.args.index(name)
	return if i >= 0 && i + 1 < os.args.len { os.args[i + 1] } else { def }
}

fn main() {
	mut tw, mut th := term.get_terminal_size()
	mut cols, mut rows := tw, th
	size := arg('--size', '')
	if size.contains('x') {
		cols, rows = size.all_before('x').int(), size.all_after('x').int()
		// A terminal that started the program before its first layout is still at its first
		// size (80x24): give it up to 3 s to reach the window's.
		for i := 0; (tw < cols || th < rows) && i < 60; i++ {
			time.sleep(50 * time.millisecond)
			tw, th = term.get_terminal_size()
		}
	}
	seconds := arg('--seconds', '15').f64()
	C.SetConsoleOutputCP(65001)
	mut f := new_fire(cols, rows)
	os.fd_write(1, '\x1b[?1049h\x1b[?25l')
	sw := time.new_stopwatch()
	mut frames, mut bytes := 0, u64(0)
	for sw.elapsed().seconds() < seconds {
		f.step()
		f.frame()
		os.fd_write(1, unsafe { tos(f.out.data, f.out.len) })
		frames++
		bytes += u64(f.out.len)
	}
	took := sw.elapsed().seconds()
	os.fd_write(1, '\x1b[0m\x1b[?25h\x1b[?1049l')
	line := 'fire: ${cols}x${rows}, ${frames} frames in ${took:.1f} s: ${f64(frames) / took:.1f} fps, ${bytes / u64(frames) / 1024} KiB/frame, ${f64(bytes) / took / 1048576:.1f} MiB/s'
	println(line)
	report := arg('--report', '')
	if report != '' {
		os.write_file(report, '${line}, terminal ${tw}x${th}, display ${display_hz()} Hz\n') or {
			eprintln('fire: ${err}')
		}
	}
}
