// Win32 message handling for Window.
module win

import quadra.platform

fn C.WaitForSingleObject(h voidptr, ms u32) u32

fn (mut w WinWindow) handle(msg u32, wparam usize, lparam isize) isize {
	if w.custom {
		if r := w.titlebar_msg(msg, wparam, lparam) {
			return r
		}
	}
	match msg {
		wm_size {
			w.width, w.height = int(lparam & 0xFFFF), int((lparam >> 16) & 0xFFFF)
			w.emit(platform.ResizeEvent{f32(w.width) / w.scale_, f32(w.height) / w.scale_})
			w.frame_now_if_modal()
			return 0
		}
		wm_paint {
			C.ValidateRect(w.hwnd, unsafe { nil })
			// In the size loop WM_SIZE has drawn this size: draw only pending changes.
			if !w.in_size_move {
				w.frame_requested = true
			}
			w.frame_now_if_modal()
			return 0
		}
		wm_erasebkgnd {
			return 1
		}
		wm_entersizemove {
			w.in_size_move = true
		}
		wm_exitsizemove {
			w.in_size_move = false
		}
		wm_dpichanged {
			w.scale_ = f32((wparam >> 16) & 0xFFFF) / 96.0
			r := unsafe { &C.RECT(voidptr(lparam)) }
			C.SetWindowPos(w.hwnd, unsafe { nil }, r.left, r.top, r.right - r.left,
				r.bottom - r.top, swp_nozorder | swp_noactivate)
			w.emit(platform.ScaleEvent{w.scale_})
			return 0
		}
		wm_close {
			w.emit(platform.CloseRequested{})
			return 0
		}
		0x0233 { // WM_DROPFILES
			if w.plat.shell.query != unsafe { nil } {
				w.emit(w.dropped_files(voidptr(wparam)))
			}
			return 0
		}
		wm_destroy {
			w.uia_disconnect()
			w.closed = true
			w.emit(platform.WindowClosed{})
			return 0
		}
		0x003D { // WM_GETOBJECT
			if r := w.uia_get_object(wparam, lparam) {
				return r
			}
		}
		0x001A, 0x0320 { // WM_SETTINGCHANGE, WM_DWMCOLORIZATIONCOLORCHANGED
			w.emit(platform.AppearanceChanged{})
		}
		wm_setcursor {
			if lparam & 0xFFFF == 1 { // HTCLIENT
				apply_cursor(w.cursor)
				return 1
			}
		}
		wm_setfocus, wm_killfocus {
			w.emit(platform.FocusEvent{msg == wm_setfocus})
			return 0
		}
		wm_keydown, wm_syskeydown, wm_keyup, wm_syskeyup {
			down := msg == wm_keydown || msg == wm_syskeydown
			w.emit(platform.KeyEvent{
				key:    key_from_vk(u32(wparam))
				code:   u32(wparam)
				mods:   current_mods()
				down:   down
				repeat: down && (u64(lparam) & (u64(1) << 30)) != 0
			})
			if msg == wm_syskeydown || msg == wm_syskeyup {
				return C.DefWindowProcW(w.hwnd, msg, wparam, lparam) // keep alt+f4 and alt+space
			}
			return 0
		}
		wm_char {
			w.on_char(u16(wparam))
			return 0
		}
		wm_mousemove {
			if !w.tracking_leave {
				t := C.TRACKMOUSEEVENT{
					cbSize:    u32(sizeof(C.TRACKMOUSEEVENT))
					dwFlags:   tme_leave
					hwndTrack: w.hwnd
				}
				C.TrackMouseEvent(&t)
				w.tracking_leave = true
			}
			w.emit_mouse(.move, .none, lparam)
			return 0
		}
		wm_mouseleave {
			w.tracking_leave = false
			w.emit(platform.MouseEvent{
				action: .leave
			})
			return 0
		}
		wm_lbuttondown, wm_rbuttondown, wm_mbuttondown, wm_xbuttondown {
			C.SetCapture(w.hwnd)
			w.emit_mouse(.down, button_of(msg, wparam), lparam)
			return if msg == wm_xbuttondown { 1 } else { 0 }
		}
		wm_lbuttonup, wm_rbuttonup, wm_mbuttonup, wm_xbuttonup {
			C.ReleaseCapture()
			w.emit_mouse(.up, button_of(msg, wparam), lparam)
			return if msg == wm_xbuttonup { 1 } else { 0 }
		}
		wm_mousewheel, wm_mousehwheel {
			mut pt := C.POINT{lo_i16(lparam), hi_i16(lparam)}
			C.ScreenToClient(w.hwnd, &pt) // wheel positions arrive in screen coords
			lines := f32(i16(u16((wparam >> 16) & 0xFFFF))) / 120.0
			w.emit(platform.WheelEvent{
				dx:   if msg == wm_mousehwheel { lines } else { 0 }
				dy:   if msg == wm_mousewheel { lines } else { 0 }
				x:    f32(pt.x) / w.scale_
				y:    f32(pt.y) / w.scale_
				mods: current_mods()
			})
			return 0
		}
		wm_ime_setcontext {
			return C.DefWindowProcW(w.hwnd, msg, wparam, lparam & ~isc_showuicompositionwindow)
		}
		wm_ime_startcomposition {
			w.emit(platform.ImeEvent{
				kind: .start
			})
			return 0
		}
		wm_ime_composition {
			for e in w.plat.ime.on_composition(w.hwnd, lparam) {
				w.emit(e)
			}
			return 0
		}
		wm_ime_endcomposition {
			w.emit(platform.ImeEvent{
				kind: .end
			})
			return 0
		}
		else {}
	}

	return C.DefWindowProcW(w.hwnd, msg, wparam, lparam)
}

// During the modal move/size loop our run() is not pumping, so frames are produced here,
// paced like run(): each waits for the swap chain's latency object first (presents that
// skip it queue up and the screen falls many frames behind the window's edge), then for
// the composition that shows it (specs/02, "Live resize").
fn (mut w WinWindow) frame_now_if_modal() {
	if w.in_size_move && w.frame_requested {
		if w.waitable != unsafe { nil } {
			C.WaitForSingleObject(w.waitable, 100)
		}
		w.frame_now()
		w.plat.after_live_frame()
	}
}

fn (mut w WinWindow) emit_mouse(action platform.MouseAction, button platform.MouseButton, lparam isize) {
	w.emit(platform.MouseEvent{
		action: action
		button: button
		x:      f32(lo_i16(lparam)) / w.scale_
		y:      f32(hi_i16(lparam)) / w.scale_
		mods:   current_mods()
	})
}

fn (mut w WinWindow) on_char(unit u16) {
	if unit >= 0xD800 && unit <= 0xDBFF {
		w.high_surrogate = unit
		return
	}
	mut units := []u16{}
	if unit >= 0xDC00 && unit <= 0xDFFF {
		if w.high_surrogate == 0 {
			return
		}
		units << w.high_surrogate
		w.high_surrogate = 0
	} else if unit < 0x20 || unit == 0x7F {
		return
	}
	units << unit
	w.emit(platform.TextEvent{utf16_to_string(&u16(units.data), units.len)})
}
