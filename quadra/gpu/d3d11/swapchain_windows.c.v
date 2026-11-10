// Flip-model swap chain with a frame-latency waitable object (lowest input latency).
module d3d11

import quadra.prim
import quadra.platform.win

type FnCreateSwapChainForHwnd = fn (this voidptr, device voidptr, hwnd voidptr, desc &SwapChainDesc1, fs voidptr, output voidptr, out &voidptr) int

type FnMakeWindowAssociation = fn (this voidptr, hwnd voidptr, flags u32) int

type FnPresent = fn (this voidptr, sync u32, flags u32) int

type FnGetBuffer = fn (this voidptr, index u32, riid &win.Guid, out &voidptr) int

type FnResizeBuffers = fn (this voidptr, count u32, w u32, h u32, format u32, flags u32) int

type FnGetHandle = fn (this voidptr) voidptr

type FnSetBackground = fn (this voidptr, rgba &f32) int

pub struct SwapChain {
pub mut:
	rtv      voidptr // render target view of the current back buffer
	width    u32
	height   u32
	waitable voidptr // signaled when the swap chain can accept a new frame
mut:
	sc voidptr     // IDXGISwapChain2
	bg ?prim.Color // last background colour given to DXGI
}

pub fn new_swapchain(d &Device, hwnd voidptr, width u32, height u32) !SwapChain {
	factory := d.dxgi_factory()!
	defer {
		win.release(factory)
	}
	desc := SwapChainDesc1{
		width:        width
		height:       height
		format:       format_b8g8r8a8_unorm
		buffer_usage: dxgi_usage_render_target_output
		buffer_count: 2
		scaling:      1 // DXGI_SCALING_NONE: no blurry stretch while resizing
		swap_effect:  dxgi_swap_effect_flip_discard
		alpha_mode:   dxgi_alpha_ignore
		flags:        dxgi_flag_frame_latency_waitable
	}
	create := unsafe {
		FnCreateSwapChainForHwnd(win.slot(factory, dxgi_factory2_create_swapchain_for_hwnd))
	}
	mut sc1 := unsafe { nil }
	win.check(create(factory, d.device, hwnd, &desc, unsafe { nil }, unsafe { nil }, &sc1),
		'CreateSwapChainForHwnd')!
	make_window_association := unsafe {
		FnMakeWindowAssociation(win.slot(factory, dxgi_factory_make_window_association))
	}
	make_window_association(factory, hwnd, dxgi_mwa_no_alt_enter)
	sc2 := win.query_interface(sc1, &iid_dxgi_swapchain2)!
	win.release(sc1)
	set_max_frame_latency := unsafe { FnSetU32(win.slot(sc2, sc2_set_max_frame_latency)) }
	set_max_frame_latency(sc2, 1)
	get_waitable := unsafe { FnGetHandle(win.slot(sc2, sc2_get_frame_latency_waitable)) }
	mut s := SwapChain{
		sc:       sc2
		width:    width
		height:   height
		waitable: get_waitable(sc2)
	}
	s.rtv = s.back_buffer_rtv(d)!
	return s
}

fn (s &SwapChain) back_buffer_rtv(d &Device) !voidptr {
	mut tex := unsafe { nil }
	get_buffer := unsafe { FnGetBuffer(win.slot(s.sc, sc_get_buffer)) }
	win.check(get_buffer(s.sc, 0, &iid_texture2d, &tex), 'GetBuffer')!
	defer {
		win.release(tex)
	}
	return d.create_rtv(tex)
}

pub fn (mut s SwapChain) resize(d &Device, width u32, height u32) ! {
	if width == 0 || height == 0 || (width == s.width && height == s.height) {
		return
	}
	win.release(s.rtv)
	s.rtv = unsafe { nil }
	resize_buffers := unsafe { FnResizeBuffers(win.slot(s.sc, sc_resize_buffers)) }
	win.check(resize_buffers(s.sc, 0, width, height, format_unknown,
		dxgi_flag_frame_latency_waitable), 'ResizeBuffers')!
	s.width, s.height = width, height
	s.rtv = s.back_buffer_rtv(d)!
}

pub fn (s &SwapChain) present() ! {
	present := unsafe { FnPresent(win.slot(s.sc, sc_present)) }
	win.check(present(s.sc, 1, 0), 'Present')!
}

// set_background gives DXGI the colour of the window area no buffer covers: while the
// window grows faster than frames arrive, the gap shows the canvas colour, not black.
pub fn (mut s SwapChain) set_background(c prim.Color) {
	if bg := s.bg {
		if bg == c {
			return
		}
	}
	s.bg = c
	rgba := [c.r, c.g, c.b, f32(1)]! // alpha is ignored (dxgi_alpha_ignore)
	set := unsafe { FnSetBackground(win.slot(s.sc, sc1_set_background_color)) }
	set(s.sc, &rgba[0])
}
