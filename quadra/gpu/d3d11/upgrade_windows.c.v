// WARP-first start-up. A cold hardware driver start takes 100+ ms, WARP about 10 ms, so
// the first frames draw on WARP while the hardware device is created on another thread.
// Once it is ready, try_upgrade moves the backend to it: the atlas pages are rebuilt by
// replaying the uploads made so far and each window's swap chain is created again.
module d3d11

import sync.stdatomic
import quadra.gpu
import quadra.platform.win

fn C.IsWindow(hwnd voidptr) int

// start_device creates a device and its pipeline on a worker thread; then (warp_first)
// the hardware one into next. One after the other: a hardware driver start holds the DLL
// loader lock for 100+ ms, which would stall WARP's start too.
fn start_device(warp bool, debug bool, mut next PendingDevice) &PendingDevice {
	mut p := &PendingDevice{}
	p.wg.add(1)
	spawn fn (mut p PendingDevice, warp bool, debug bool, mut next PendingDevice) {
		p.fill(warp, debug)
		if next != unsafe { nil } {
			next.fill(false, debug)
		}
	}(mut p, warp, debug, mut next)
	return p
}

fn (mut p PendingDevice) fill(warp bool, debug bool) {
	p.dev = new_device(force_warp: warp, debug: debug) or {
		p.err = err.msg()
		unsafe { nil }
	}
	if p.err == '' {
		// Shaders and states are device objects (free-threaded), so they load here too.
		p.pipe = new_pipeline(p.dev) or {
			p.err = err.msg()
			Pipeline{}
		}
	}
	stdatomic.store_u64(&p.done, 1)
	p.wg.done()
}

// upgrading reports whether a hardware device is still on its way.
pub fn (b &Backend) upgrading() bool {
	return b.upgrade != unsafe { nil }
}

// try_upgrade switches to the hardware device once it is ready. True when it is over (it
// switched or stays): every window must draw again, and that draw gives the window its
// swap chain (attach), so its waitable changes.
pub fn (mut b Backend) try_upgrade() bool {
	mut p := b.upgrade
	if p == unsafe { nil } || stdatomic.load_u64(&p.done) == 0 {
		return false
	}
	b.upgrade = unsafe { nil }
	replay, windows := b.replay, b.windows
	b.replay, b.windows = []gpu.Upload{}, []&WindowSurface{}
	b.ready() or { return false }
	// No hardware (the device fell back to WARP) or offscreen surfaces bound to the old
	// device (they are never moved): stay on WARP.
	if p.err != '' || p.dev.warp || b.offscreens > 0 {
		release_device(p.dev, mut p.pipe)
		return true
	}
	for s in windows {
		mut m := unsafe { s }
		if !m.gdi {
			continue
		}
		off := new_offscreen(p.dev, m.off.width, m.off.height) or {
			eprintln('quadra: hardware GPU switch failed, staying on WARP: ${err}')
			release_device(p.dev, mut p.pipe)
			return true
		}
		m.off.release()
		m.off, m.dev = off, p.dev
	}
	for pg in b.mono {
		win.release(pg.srv)
		win.release(pg.tex)
	}
	for pg in b.poly {
		win.release(pg.srv)
		win.release(pg.tex)
	}
	b.mono, b.poly = []AtlasPage{}, []AtlasPage{}
	release_device(b.dev, mut b.pipe)
	b.dev, b.pipe = p.dev, p.pipe
	for u in replay {
		b.upload(u) or { eprintln('quadra: atlas upload failed: ${err}') }
	}
	return true
}

// attach gives a window drawn through GDI its swap chain, at the start of a draw (made
// from a timer instead, between frames, the frames ran late now and then for good). On
// failure the window keeps GDI presents.
fn (mut s WindowSurface) attach() {
	if C.IsWindow(s.hwnd) == 0 {
		return
	}
	s.sc = new_swapchain(s.dev, s.hwnd, s.off.width, s.off.height) or {
		eprintln('quadra: cannot create the swap chain, drawing through GDI: ${err}')
		return
	}
	s.off.release()
	s.off = Offscreen{}
	s.gdi = false
}

fn (mut s SwapChain) release(d &Device) {
	win.release(s.rtv)
	win.release(s.sc)
	if s.waitable != unsafe { nil } {
		C.CloseHandle(s.waitable)
	}
	s.rtv, s.sc, s.waitable = unsafe { nil }, unsafe { nil }, unsafe { nil }
	d.unbind_all() // the back buffers go only once the context lets go of them
}

fn (mut p Pipeline) release() {
	for o in [p.vs, p.ps, p.blend, p.raster, p.sampler, p.globals, p.instances, p.srv, p.xforms,
		p.xsrv] {
		win.release(o)
	}
	p = Pipeline{}
}

fn release_device(d &Device, mut pipe Pipeline) {
	if d == unsafe { nil } {
		return
	}
	pipe.release()
	d.unbind_all()
	win.release(d.ctx)
	win.release(d.device)
}
