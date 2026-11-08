// Custom titlebar: the client area covers the caption, the resize borders stay with the
// OS, and the app's titlebar regions (EventSink.hit_test) become the caption and window
// buttons. Returning HTMAXBUTTON over the app's maximize button gives Windows 11 snap
// layouts.
module win

import quadra.platform

fn C.IsZoomed(hwnd voidptr) int

const wm_nccalcsize = u32(0x0083)
const wm_nchittest = u32(0x0084)
const wm_ncmousemove = u32(0x00A0)
const wm_nclbuttondown = u32(0x00A1)
const wm_nclbuttonup = u32(0x00A2)
const wm_ncmouseleave = u32(0x02A2)
const ht_client = isize(1)
const ht_caption = isize(2)
const ht_minbutton = isize(8)
const ht_maxbutton = isize(9)
const ht_left = isize(10)
const ht_right = isize(11)
const ht_top = isize(12)
const ht_topleft = isize(13)
const ht_topright = isize(14)
const ht_close = isize(20)
const tme_nonclient = u32(0x0010)

pub fn (w &WinWindow) is_maximized() bool {
	return C.IsZoomed(w.hwnd) != 0
}

pub fn (mut w WinWindow) minimize() {
	C.ShowWindow(w.hwnd, 6) // SW_MINIMIZE
}

pub fn (mut w WinWindow) toggle_maximize() {
	cmd := if w.is_maximized() { 9 } else { 3 } // SW_RESTORE, SW_MAXIMIZE
	C.ShowWindow(w.hwnd, cmd)
}

// frame_y is the height of the top resize border in device px.
fn (w &WinWindow) frame_y() int {
	return int(8 * w.scale_)
}

// titlebar_msg handles the messages a custom titlebar changes; none means "not handled".
fn (mut w WinWindow) titlebar_msg(msg u32, wparam usize, lparam isize) ?isize {
	match msg {
		wm_nccalcsize {
			if wparam == 0 {
				return none
			}
			// Let Windows compute the standard client rect, then give the caption back to
			// the client. Maximized windows overhang the screen by the frame: keep it out.
			mut r := unsafe { &C.RECT(voidptr(lparam)) }
			top := r.top
			C.DefWindowProcW(w.hwnd, msg, wparam, lparam)
			inset := if w.is_maximized() { w.frame_y() } else { 0 }
			r.top = top + inset
			return 0
		}
		wm_nchittest {
			hit := C.DefWindowProcW(w.hwnd, msg, wparam, lparam)
			mut pt := C.POINT{int(i16(lparam & 0xFFFF)), int(i16((lparam >> 16) & 0xFFFF))}
			C.ScreenToClient(w.hwnd, &pt)
			if hit != ht_client {
				// Side borders reach into the top band the caption took: make those corners
				// diagonal, as on a standard frame.
				if !w.is_maximized() && pt.y < w.frame_y() {
					if hit == ht_left {
						return ht_topleft
					}
					if hit == ht_right {
						return ht_topright
					}
				}
				return hit // left, right and bottom resize borders
			}
			if !w.is_maximized() && pt.y < w.frame_y() {
				return ht_top
			}
			mut p := w.plat
			region := p.sink.hit_test(w.id, f32(pt.x) / w.scale_, f32(pt.y) / w.scale_)
			return match region {
				.client { ht_client }
				.caption { ht_caption }
				.minimize { ht_minbutton }
				.maximize { ht_maxbutton }
				.close { ht_close }
			}
		}
		wm_ncmousemove {
			// Over the window buttons the OS sends non-client moves: keep hover styles live.
			if !w.tracking_leave {
				t := C.TRACKMOUSEEVENT{
					cbSize:    u32(sizeof(C.TRACKMOUSEEVENT))
					dwFlags:   tme_leave | tme_nonclient
					hwndTrack: w.hwnd
				}
				C.TrackMouseEvent(&t)
				w.tracking_leave = true
			}
			w.emit_mouse(.move, .none, w.client_lparam(lparam))
			return none
		}
		wm_ncmouseleave {
			w.tracking_leave = false
			w.emit(platform.MouseEvent{
				action: .leave
			})
			return 0
		}
		wm_nclbuttondown {
			if isize(wparam) in [ht_minbutton, ht_maxbutton, ht_close] {
				w.nc_down = isize(wparam)
				return 0 // no classic button drawing
			}
			return none
		}
		wm_nclbuttonup {
			code := isize(wparam)
			if code == w.nc_down && code != 0 {
				w.nc_down = 0
				match code {
					ht_minbutton { w.minimize() }
					ht_maxbutton { w.toggle_maximize() }
					else { C.PostMessageW(w.hwnd, wm_close, 0, 0) }
				}

				return 0
			}
			w.nc_down = 0
			return none
		}
		else {
			return none
		}
	}
}

// client_lparam converts a screen-coordinate lparam to client coordinates.
fn (w &WinWindow) client_lparam(lparam isize) isize {
	mut pt := C.POINT{int(i16(lparam & 0xFFFF)), int(i16((lparam >> 16) & 0xFFFF))}
	C.ScreenToClient(w.hwnd, &pt)
	return isize((u32(u16(pt.y)) << 16) | u32(u16(pt.x)))
}
