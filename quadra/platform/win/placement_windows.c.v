// Window placement (restore position, size and maximized) and dropped files.
module win

import quadra.platform

@[typedef]
pub struct C.WINDOWPLACEMENT {
pub mut:
	length           u32
	flags            u32
	showCmd          u32
	ptMinPosition    C.POINT
	ptMaxPosition    C.POINT
	rcNormalPosition C.RECT
}

fn C.GetWindowPlacement(hwnd voidptr, wp &C.WINDOWPLACEMENT) int
fn C.SetWindowPlacement(hwnd voidptr, wp &C.WINDOWPLACEMENT) int
fn C.MonitorFromRect(r &C.RECT, flags u32) voidptr

const sw_showmaximized = 3
const sw_shownormal = 1

pub fn (w &WinWindow) placement() platform.Placement {
	mut wp := C.WINDOWPLACEMENT{
		length: u32(sizeof(C.WINDOWPLACEMENT))
	}
	if C.GetWindowPlacement(w.hwnd, &wp) == 0 {
		return platform.Placement{}
	}
	r := wp.rcNormalPosition
	return platform.Placement{
		x:         r.left
		y:         r.top
		width:     r.right - r.left
		height:    r.bottom - r.top
		maximized: wp.showCmd == sw_showmaximized
		valid:     true
	}
}

// restore_placement applies a saved placement when its frame still touches a monitor
// (monitors may have been unplugged since). Returns the show command to use.
fn restore_placement(hwnd voidptr, p platform.Placement) int {
	if !p.valid || p.width <= 0 || p.height <= 0 {
		return sw_shownormal
	}
	r := C.RECT{p.x, p.y, p.x + p.width, p.y + p.height}
	if C.MonitorFromRect(&r, 0) == unsafe { nil } { // MONITOR_DEFAULTTONULL
		return sw_shownormal
	}
	show := if p.maximized { sw_showmaximized } else { sw_shownormal }
	wp := C.WINDOWPLACEMENT{
		length:           u32(sizeof(C.WINDOWPLACEMENT))
		showCmd:          u32(show)
		rcNormalPosition: r
	}
	C.SetWindowPlacement(hwnd, &wp)
	return show
}

type FnDragAcceptFiles = fn (hwnd voidptr, accept int)

type FnDragQueryFileW = fn (drop voidptr, i u32, buf &u16, n u32) u32

type FnDragQueryPoint = fn (drop voidptr, pt &C.POINT) int

type FnDragFinish = fn (drop voidptr)

struct Shell32 {
	accept FnDragAcceptFiles = unsafe { nil }
	query  FnDragQueryFileW  = unsafe { nil }
	point  FnDragQueryPoint  = unsafe { nil }
	finish FnDragFinish      = unsafe { nil }
	// tray_windows.c.v
	notify_icon FnShellNotifyIcon = unsafe { nil }
}

fn load_shell32() Shell32 {
	lib := load_system_dll('shell32.dll') or { return Shell32{} }
	return Shell32{
		accept:      FnDragAcceptFiles(dll_proc(lib, 'DragAcceptFiles') or { return Shell32{} })
		query:       FnDragQueryFileW(dll_proc(lib, 'DragQueryFileW') or { return Shell32{} })
		point:       FnDragQueryPoint(dll_proc(lib, 'DragQueryPoint') or { return Shell32{} })
		finish:      FnDragFinish(dll_proc(lib, 'DragFinish') or { return Shell32{} })
		notify_icon: FnShellNotifyIcon(dll_proc(lib, 'Shell_NotifyIconW') or { return Shell32{} })
	}
}

// dropped_files reads a WM_DROPFILES handle into an event (positions in logical px).
fn (w &WinWindow) dropped_files(drop voidptr) platform.FileDropEvent {
	s := w.plat.shell
	n := s.query(drop, 0xFFFFFFFF, unsafe { nil }, 0)
	mut paths := []string{cap: int(n)}
	for i in 0 .. n {
		len := s.query(drop, i, unsafe { nil }, 0)
		mut buf := []u16{len: int(len) + 1}
		s.query(drop, i, buf.data, len + 1)
		paths << unsafe { string_from_wide2(buf.data, int(len)) }
	}
	mut pt := C.POINT{}
	s.point(drop, &pt)
	s.finish(drop)
	return platform.FileDropEvent{paths, f32(pt.x) / w.scale_, f32(pt.y) / w.scale_}
}
