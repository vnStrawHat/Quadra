// Backend implements gpu.Device on D3D11: one device shared by every window,
// atlas pages as textures, and window/offscreen surfaces.
module d3d11

import sync
import quadra.prim
import quadra.gpu

struct AtlasPage {
mut:
	tex voidptr
	srv voidptr
}

@[heap]
pub struct Backend {
mut:
	dev       &Device = unsafe { nil }
	pipe      Pipeline
	mono      []AtlasPage
	poly      []AtlasPage
	page_size int
	st        gpu.RenderStats
	pending   &PendingDevice = unsafe { nil }
	// WARP-first start-up (upgrade_windows.c.v): the hardware device on its way, and what
	// moves to it.
	upgrade    &PendingDevice = unsafe { nil }
	replay     []gpu.Upload
	windows    []&WindowSurface
	offscreens int
}

// PendingDevice is a device being created on a worker thread.
@[heap]
struct PendingDevice {
mut:
	wg   &sync.WaitGroup = sync.new_waitgroup()
	dev  &Device         = unsafe { nil }
	pipe Pipeline
	err  string
	done u64 // set (atomically) when dev or err is
}

@[params]
pub struct BackendOptions {
pub:
	force_warp bool
	// warp_first draws on WARP until the hardware device is up (try_upgrade).
	warp_first bool
	debug      bool
	page_size  int = 2048
}

pub fn new_backend(opts BackendOptions) !&Backend {
	dev := new_device(force_warp: opts.force_warp, debug: opts.debug)!
	return &Backend{
		dev:       dev
		pipe:      new_pipeline(dev)!
		page_size: opts.page_size
	}
}

// new_backend_async starts creating the device on a worker thread and returns at once:
// a cold driver start takes 100+ ms on many machines, which then overlaps with window
// creation and the first layout. The first call that needs the device waits for it.
pub fn new_backend_async(opts BackendOptions) &Backend {
	warp_first := opts.warp_first && !opts.force_warp
	mut b := &Backend{
		page_size: opts.page_size
	}
	if warp_first {
		mut up := &PendingDevice{}
		up.wg.add(1)
		b.upgrade = up
	}
	b.pending = start_device(opts.force_warp || warp_first, opts.debug, mut b.upgrade)
	return b
}

// ready waits for an asynchronously created device (no-op otherwise).
fn (mut b Backend) ready() ! {
	if b.dev != unsafe { nil } {
		return
	}
	mut p := b.pending
	p.wg.wait()
	if p.err != '' {
		return error(p.err)
	}
	b.dev = p.dev
	b.pipe = p.pipe
}

pub fn (b &Backend) is_software() bool {
	mut m := unsafe { b }
	m.ready() or { return false }
	return b.dev.warp
}

// adapter_name is the description of the adapter the current device draws on.
pub fn (b &Backend) adapter_name() string {
	mut m := unsafe { b }
	m.ready() or { return '' }
	return b.dev.adapter
}

pub fn (b &Backend) stats() gpu.RenderStats {
	mut st := b.st
	st.resident_bytes = i64(b.pipe.capacity) * i64(sizeof(gpu.Instance)) +
		i64(b.pipe.xcap) * i64(sizeof(gpu.Rigid))
	for i in 0 .. b.mono.len {
		e := i64(gpu.page_extent(b.page_size, i))
		st.resident_bytes += e * e
	}
	for i in 0 .. b.poly.len {
		e := i64(gpu.page_extent(b.page_size, i))
		st.resident_bytes += 4 * e * e
	}
	return st
}

fn (mut b Backend) page(kind gpu.AtlasKind, index int) !AtlasPage {
	for {
		pages := if kind == .mono { b.mono } else { b.poly }
		if index < pages.len {
			return pages[index]
		}
		size := u32(gpu.page_extent(b.page_size, pages.len))
		tex := b.dev.create_texture2d(&Texture2dDesc{
			width:      size
			height:     size
			format:     if kind == .mono { format_r8_unorm } else { format_r8g8b8a8_unorm }
			usage:      usage_default
			bind_flags: bind_shader_resource
		}, unsafe { nil })!
		p := AtlasPage{tex, b.dev.create_srv(tex, unsafe { nil })!}
		if kind == .mono {
			b.mono << p
		} else {
			b.poly << p
		}
	}
	return error('unreachable')
}

pub fn (mut b Backend) upload(u gpu.Upload) ! {
	b.ready()!
	p := b.page(u.kind, u.page)!
	bpp := if u.kind == .mono { 1 } else { 4 }
	b.dev.update_region(p.tex, Box{
		left:   u32(u.x)
		top:    u32(u.y)
		right:  u32(u.x + u.w)
		bottom: u32(u.y + u.h)
	}, u.pixels.data, u32(u.w * bpp))
	b.st.upload_bytes += u.pixels.len
	if b.upgrade != unsafe { nil } {
		b.replay << u
	}
}

pub fn (mut b Backend) new_window_surface(native_handle voidptr, width int, height int) !gpu.Surface {
	b.ready()!
	if native_handle == unsafe { nil } {
		// Headless windows have no HWND: render offscreen (pixel snapshots).
		return b.new_offscreen_surface(width, height)
	}
	if b.upgrade != unsafe { nil } {
		s := &WindowSurface{
			dev:  b.dev
			hwnd: native_handle
			gdi:  true
			off:  new_offscreen(b.dev, u32(width), u32(height))!
		}
		b.windows << s
		return s
	}
	return &WindowSurface{
		dev:  b.dev
		sc:   new_swapchain(b.dev, native_handle, u32(width), u32(height))!
		hwnd: native_handle
	}
}

pub fn (mut b Backend) new_offscreen_surface(width int, height int) !gpu.Surface {
	b.ready()!
	b.offscreens++
	return &OffscreenSurface{
		dev: b.dev
		off: new_offscreen(b.dev, u32(width), u32(height))!
	}
}

pub fn (mut b Backend) draw(mut surface gpu.Surface, scene &gpu.Scene, clear prim.Color) ! {
	rtv := match mut surface {
		WindowSurface {
			if surface.gdi && b.upgrade == unsafe { nil } {
				surface.attach()
			}
			if !surface.gdi {
				surface.sc.set_background(clear)
			}
			surface.rtv()
		}
		OffscreenSurface {
			surface.off.rtv
		}
		else {
			return error('surface does not belong to the d3d11 backend')
		}
	}

	b.ready()!
	// Make sure every page referenced by the scene exists, even if empty (the batches list
	// the pages, so this does not walk every instance).
	for bt in scene.batches() {
		if bt.mono_page >= 0 {
			b.page(.mono, bt.mono_page)!
		}
		if bt.poly_page >= 0 {
			b.page(.poly, bt.poly_page)!
		}
	}
	// Clear with premultiplied color.
	b.dev.clear(rtv, clear.r * clear.a, clear.g * clear.a, clear.b * clear.a, clear.a)
	calls := b.pipe.draw(b.dev, rtv, surface.width(), surface.height(), scene, b.mono, b.poly)!
	b.st.frames++
	b.st.draw_calls = calls
	b.st.instances = scene.instances.len
}
