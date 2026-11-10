// WinWindow: platform.PlatformWindow on Win32.
module win

import quadra.prim
import quadra.platform

@[heap]
pub struct WinWindow {
pub:
	id platform.WindowId
pub mut:
	hwnd     voidptr
	scale_   f32 = 1.0
	width    int // client size, device px
	height   int
	waitable voidptr
	closed   bool
mut:
	plat            &WinPlatform = unsafe { nil }
	frame_requested bool
	high_surrogate  u16
	tracking_leave  bool
	in_size_move    bool
	cursor          platform.Cursor
	ime_enabled     bool = true
	custom          bool  // custom titlebar (titlebar_windows.c.v)
	nc_down         isize // window button pressed in the non-client area
	uia             &Uia = unsafe { nil } // uia_windows.c.v, made when UIA first asks
}

const class_name = 'QuadraWindow'

fn (mut p WinPlatform) create_window(opts platform.WindowOptions) !&WinWindow {
	p.next_id++
	mut w := &WinWindow{
		id:     p.next_id
		plat:   p
		custom: opts.custom_titlebar
	}
	inst := C.GetModuleHandleW(unsafe { nil })
	wc := C.WNDCLASSEXW{
		cbSize:        u32(sizeof(C.WNDCLASSEXW))
		style:         cs_hredraw | cs_vredraw
		lpfnWndProc:   voidptr(wnd_proc)
		hInstance:     inst
		hCursor:       unsafe { nil }
		lpszClassName: class_name.to_wide()
		lpszMenuName:  unsafe { nil }
	}
	C.RegisterClassExW(&wc) // fails harmlessly when already registered
	style := if opts.resizable {
		ws_overlappedwindow
	} else {
		ws_overlappedwindow & ~u32(0x00050000)
	}
	hwnd := C.CreateWindowExW(0, class_name.to_wide(), opts.title.to_wide(), style, cw_usedefault,
		cw_usedefault, int(opts.width), int(opts.height), unsafe { nil }, unsafe { nil }, inst,
		voidptr(w))
	if hwnd == unsafe { nil } {
		return error('CreateWindowExW failed: ${C.GetLastError()}')
	}
	// Size the client area to the requested logical size at this monitor's DPI.
	w.scale_ = p.dpi.scale_of(hwnd)
	mon := primary_display()
	mw, mh := mon.right - mon.left, mon.bottom - mon.top
	size := platform.fit_size(opts.width, opts.height, prim.size(f32(mw) / w.scale_,
		f32(mh) / w.scale_), opts.fit)
	mut r := C.RECT{0, 0, int(size.width * w.scale_), int(size.height * w.scale_)}
	p.dpi.adjust_rect(&r, style, 0, 0, u32(w.scale_ * 96))
	ow, oh := r.right - r.left, r.bottom - r.top
	mut flags := swp_nozorder | swp_noactivate
	if !opts.centered {
		flags |= 0x0002 // SWP_NOMOVE
	}
	C.SetWindowPos(hwnd, unsafe { nil }, mon.left + (mw - ow) / 2, mon.top + (mh - oh) / 2, ow, oh,
		flags)
	mut cr := C.RECT{}
	C.GetClientRect(hwnd, &cr)
	if w.custom {
		C.SetWindowPos(hwnd, unsafe { nil }, 0, 0, 0, 0,
			swp_nozorder | swp_noactivate | 0x0001 | 0x0002 | 0x0020) // NOSIZE NOMOVE FRAMECHANGED
	}
	show := restore_placement(hwnd, opts.placement)
	C.GetClientRect(hwnd, &cr)
	w.width, w.height = cr.right, cr.bottom
	if p.shell.accept != unsafe { nil } {
		p.shell.accept(hwnd, 1)
	}
	cmd := if show == sw_showmaximized { show } else { 5 } // SW_SHOW keeps the OS default
	C.ShowWindow(hwnd, cmd)
	return w
}

pub fn (w &WinWindow) size() prim.Size {
	return prim.size(f32(w.width) / w.scale_, f32(w.height) / w.scale_)
}

pub fn (w &WinWindow) scale() f32 {
	return w.scale_
}

pub fn (w &WinWindow) native_handle() voidptr {
	return w.hwnd
}

pub fn (w &WinWindow) set_title(title string) {
	C.SetWindowTextW(w.hwnd, title.to_wide())
}

pub fn (mut w WinWindow) set_cursor(c platform.Cursor) {
	if w.cursor != c {
		w.cursor = c
		apply_cursor(c)
	}
}

pub fn (mut w WinWindow) request_frame() {
	w.frame_requested = true
}

pub fn (mut w WinWindow) set_ime_rect(r prim.Rect) {
	s := w.scale_
	w.plat.ime.set_caret(w.hwnd, C.RECT{int(r.x * s), int(r.y * s), int((r.x + r.width) * s), int((
		r.y + r.height) * s)})
}

pub fn (mut w WinWindow) enable_ime(on bool) {
	if w.ime_enabled != on {
		w.ime_enabled = on
		w.plat.ime.enable(w.hwnd, on)
	}
}

pub fn (mut w WinWindow) set_waitable(handle voidptr) {
	w.waitable = handle
}

pub fn (mut w WinWindow) close() {
	if !w.closed {
		C.DestroyWindow(w.hwnd)
	}
}

pub fn (w &WinWindow) is_closed() bool {
	return w.closed
}

// post_char_units queues raw UTF-16 WM_CHAR messages to this window only (test aid).
pub fn (w &WinWindow) post_char_units(units []u16) {
	for u in units {
		C.PostMessageW(w.hwnd, wm_char, usize(u), 1)
	}
}

pub fn (w &WinWindow) post_close() {
	C.PostMessageW(w.hwnd, wm_close, 0, 0)
}

fn (mut w WinWindow) emit(e platform.Event) {
	w.plat.sink.on_event(w.id, e)
}

fn (mut w WinWindow) frame_now() {
	w.frame_requested = false
	w.plat.sink.on_frame(w.id)
}

fn wnd_proc(hwnd voidptr, msg u32, wparam usize, lparam isize) isize {
	if msg == wm_nccreate {
		cs := unsafe { &C.CREATESTRUCTW(voidptr(lparam)) }
		C.SetWindowLongPtrW(hwnd, gwlp_userdata, isize(cs.lpCreateParams))
		mut w := unsafe { &WinWindow(cs.lpCreateParams) }
		w.hwnd = hwnd
		return C.DefWindowProcW(hwnd, msg, wparam, lparam)
	}
	p := C.GetWindowLongPtrW(hwnd, gwlp_userdata)
	if p == 0 {
		return C.DefWindowProcW(hwnd, msg, wparam, lparam)
	}
	mut w := unsafe { &WinWindow(voidptr(p)) }
	return w.handle(msg, wparam, lparam)
}
