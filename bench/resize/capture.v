// Screen readback with GDI (what DWM composed) and DwmFlush, both loaded at run time:
// linking gdi32 would make V build a console-less program.
module main

import stbi
import time
import quadra.platform.win

fn C.GetDC(hwnd voidptr) voidptr
fn C.ReleaseDC(hwnd voidptr, hdc voidptr) int
fn C.ClientToScreen(hwnd voidptr, pt &C.POINT) int
fn C.GetWindowRect(hwnd voidptr, r &C.RECT) int
fn C.GetSystemMetrics(i int) int
fn C.SetForegroundWindow(hwnd voidptr) int
fn C.SendMessageW(hwnd voidptr, msg u32, wparam usize, lparam isize) isize

type FnCreateDC = fn (hdc voidptr) voidptr

type FnCreateBitmap = fn (hdc voidptr, w int, h int) voidptr

type FnSelect = fn (hdc voidptr, obj voidptr) voidptr

type FnBitBlt = fn (dst voidptr, x int, y int, w int, h int, src voidptr, sx int, sy int, rop u32) int

type FnGetDIBits = fn (hdc voidptr, bmp voidptr, start u32, lines u32, bits voidptr, bmi voidptr, usage u32) int

type FnDelete = fn (obj voidptr) int

type FnFlush = fn () int

struct Bmi {
	size          u32 = 40
	width         int
	height        int
	planes        u16 = 1
	bit_count     u16 = 32
	compression   u32
	size_image    u32
	x_ppm         int
	y_ppm         int
	clr_used      u32
	clr_important u32
}

struct Capture {
	create_dc  FnCreateDC     = unsafe { nil }
	create_bmp FnCreateBitmap = unsafe { nil }
	sel        FnSelect       = unsafe { nil }
	blt        FnBitBlt       = unsafe { nil }
	bits       FnGetDIBits    = unsafe { nil }
	del_obj    FnDelete       = unsafe { nil }
	del_dc     FnDelete       = unsafe { nil }
	dwm_flush  FnFlush        = unsafe { nil }
}

fn new_capture() !Capture {
	gdi := win.load_system_dll('gdi32.dll')!
	dwm := win.load_system_dll('dwmapi.dll')!
	return Capture{
		create_dc:  unsafe { FnCreateDC(win.dll_proc(gdi, 'CreateCompatibleDC')!) }
		create_bmp: unsafe { FnCreateBitmap(win.dll_proc(gdi, 'CreateCompatibleBitmap')!) }
		sel:        unsafe { FnSelect(win.dll_proc(gdi, 'SelectObject')!) }
		blt:        unsafe { FnBitBlt(win.dll_proc(gdi, 'BitBlt')!) }
		bits:       unsafe { FnGetDIBits(win.dll_proc(gdi, 'GetDIBits')!) }
		del_obj:    unsafe { FnDelete(win.dll_proc(gdi, 'DeleteObject')!) }
		del_dc:     unsafe { FnDelete(win.dll_proc(gdi, 'DeleteDC')!) }
		dwm_flush:  unsafe { FnFlush(win.dll_proc(dwm, 'DwmFlush')!) }
	}
}

fn (c &Capture) flush() {
	c.dwm_flush()
}

// grab reads a screen rectangle as top-down 0xAARRGGBB pixels.
fn (c &Capture) grab(x int, y int, w int, h int) []u32 {
	screen := C.GetDC(unsafe { nil })
	mem := c.create_dc(screen)
	bmp := c.create_bmp(screen, w, h)
	old := c.sel(mem, bmp)
	c.blt(mem, 0, 0, w, h, screen, x, y, 0x00CC0020 | 0x40000000) // SRCCOPY | CAPTUREBLT
	c.sel(mem, old)
	mut px := []u32{len: w * h}
	bmi := Bmi{
		width:  w
		height: -h
	}
	c.bits(mem, bmp, 0, u32(h), px.data, &bmi, 0)
	c.del_obj(bmp)
	c.del_dc(mem)
	C.ReleaseDC(unsafe { nil }, screen)
	return px
}

fn client_origin(hwnd voidptr) (int, int, int, int) {
	mut cr := C.RECT{}
	C.GetClientRect(hwnd, &cr)
	mut pt := C.POINT{0, 0}
	C.ClientToScreen(hwnd, &pt)
	return pt.x, pt.y, cr.right, cr.bottom
}

// pump dispatches the window's queued messages, as the modal size loop does between moves.
fn pump(hwnd voidptr) {
	msg := [6]u64{} // MSG is 48 bytes on x64
	m := unsafe { voidptr(&msg[0]) }
	for C.PeekMessageW(m, hwnd, 0, 0, 1) != 0 {
		C.TranslateMessage(m)
		C.DispatchMessageW(m)
	}
}

struct Sample {
	t     f64 // ms, QPC
	gap   int // px between the strip and the visible window edge; -1: no strip or edge
	color u32 // the first pixel right of the strip
}

@[heap]
struct Sampler {
	cap Capture
	x   int // screen region: the client area and `extra` px to its right
	y   int
	w   int
	h   int
mut:
	band      bool
	stop      bool
	settled   int // sample indices: before the loop, at its start and at its end
	started   int
	ended     int
	samples   []Sample
	worst     []u32
	worst_gap int = -1
}

fn new_sampler(hwnd voidptr, extra int) !&Sampler {
	x, y, w, h := client_origin(hwnd)
	return &Sampler{
		cap: new_capture()!
		x:   x
		y:   y
		w:   w + extra
		h:   h
	}
}

fn sample_loop(mut s Sampler) {
	sw := time.new_stopwatch()
	for !s.stop {
		// A thin band disturbs DWM less than a full readback; the full one gives the shots.
		px := if s.band {
			s.cap.grab(s.x, s.y + s.h / 2, s.w, 1)
		} else {
			s.cap.grab(s.x, s.y, s.w, s.h)
		}
		mid := if s.band { 0 } else { s.h / 2 }
		row := px[s.w * mid..s.w * (mid + 1)]
		gap, color := measure(row)
		s.samples << Sample{f64(sw.elapsed().microseconds()) / 1000, gap, color}
		if !s.band && (gap > s.worst_gap || s.worst.len == 0) {
			s.worst_gap = gap
			s.worst = px.clone()
		}
	}
}

// measure finds the visible window edge (the green backdrop starts; the shadow only darkens
// it) and the magenta strip left of it.
fn measure(row []u32) (int, u32) {
	mut e := row.len
	for e > 0 && is_backdrop(row[e - 1]) {
		e--
	}
	mut m := e - 1
	for m >= 0 && !is_marker(row[m]) {
		m--
	}
	if m < 0 || e == row.len {
		return -1, 0
	}
	return e - m - 1, if m + 1 < row.len {
		row[m + 1]
	} else {
		u32(0)
	}
}

fn is_backdrop(c u32) bool {
	r, g, b := (c >> 16) & 0xFF, (c >> 8) & 0xFF, c & 0xFF
	return g >= 40 && r < 40 && b < 40
}

fn is_marker(c u32) bool {
	r, g, b := (c >> 16) & 0xFF, (c >> 8) & 0xFF, c & 0xFF
	return r > 200 && g < 60 && b > 200
}

fn (s &Sampler) summary() {
	mut base := 1 << 30
	for p in s.samples[..s.settled] {
		if p.gap >= 0 && p.gap < base {
			base = p.gap
		}
	}
	mut worst, mut over := 0, 0
	mut colors := map[u32]int{}
	live := s.samples[s.started..s.ended]
	for p in live {
		g := p.gap - base
		if g > worst {
			worst = g
		}
		if g > 3 {
			over++
			colors[p.color & 0xFFFFFF]++
		}
	}
	dt := if live.len > 1 { (live.last().t - live[0].t) / (live.len - 1) } else { 0 }
	println('samples during the loop: ${live.len} (every ${dt:.1} ms), border ${base} px')
	println('gap > 3 px in ${over}/${live.len} samples (${f64(over) * 100 / f64(live.len):.0}%), worst ${worst} px')
	for c, n in colors {
		println('  gap colour #${c:06x}: ${n} samples')
	}
}

// save_worst writes the sample with the widest gap as a PNG.
fn (s &Sampler) save_worst(dir string) {
	if s.worst.len == 0 {
		return
	}
	mut rgba := []u8{len: s.worst.len * 4}
	for i, c in s.worst {
		rgba[i * 4] = u8(c >> 16)
		rgba[i * 4 + 1] = u8(c >> 8)
		rgba[i * 4 + 2] = u8(c)
		rgba[i * 4 + 3] = 255
	}
	path := '${dir}/worst.png'
	stbi.stbi_write_png(path, s.w, s.h, 4, rgba.data, s.w * 4) or { eprintln(err) }
	println('worst sample: ${path}')
}
