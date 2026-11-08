// Win32 declarations used by the window implementation. Only base APIs from
// user32/kernel32 are linked; newer entry points are loaded at runtime (dpi_windows.c.v).
module win

#flag windows -luser32

@[typedef]
pub struct C.POINT {
pub mut:
	x int
	y int
}

@[typedef]
pub struct C.RECT {
pub mut:
	left   int
	top    int
	right  int
	bottom int
}

@[typedef]
struct C.MSG {
mut:
	hwnd    voidptr
	message u32
	wParam  usize
	lParam  isize
	time    u32
	pt      C.POINT
}

@[typedef]
struct C.WNDCLASSEXW {
mut:
	cbSize        u32
	style         u32
	lpfnWndProc   voidptr
	cbClsExtra    int
	cbWndExtra    int
	hInstance     voidptr
	hIcon         voidptr
	hCursor       voidptr
	hbrBackground voidptr
	lpszMenuName  &u16
	lpszClassName &u16
	hIconSm       voidptr
}

@[typedef]
struct C.CREATESTRUCTW {
	lpCreateParams voidptr
}

@[typedef]
struct C.TRACKMOUSEEVENT {
mut:
	cbSize      u32
	dwFlags     u32
	hwndTrack   voidptr
	dwHoverTime u32
}

fn C.GetModuleHandleW(name &u16) voidptr
fn C.RegisterClassExW(wc &C.WNDCLASSEXW) u16
fn C.CreateWindowExW(ex u32, class &u16, title &u16, style u32, x int, y int, w int, h int, parent voidptr, menu voidptr, inst voidptr, param voidptr) voidptr
fn C.DestroyWindow(hwnd voidptr) int
fn C.ShowWindow(hwnd voidptr, cmd int) int
fn C.DefWindowProcW(hwnd voidptr, msg u32, wparam usize, lparam isize) isize
fn C.SetWindowLongPtrW(hwnd voidptr, index int, value isize) isize
fn C.GetWindowLongPtrW(hwnd voidptr, index int) isize
fn C.GetClientRect(hwnd voidptr, r &C.RECT) int
fn C.SetWindowPos(hwnd voidptr, after voidptr, x int, y int, w int, h int, flags u32) int
fn C.SetWindowTextW(hwnd voidptr, s &u16) int
fn C.PeekMessageW(msg &C.MSG, hwnd voidptr, min u32, max u32, remove u32) int
fn C.TranslateMessage(msg &C.MSG) int
fn C.DispatchMessageW(msg &C.MSG) isize
fn C.PostQuitMessage(code int)
fn C.MsgWaitForMultipleObjectsEx(count u32, handles &voidptr, ms u32, mask u32, flags u32) u32
fn C.LoadCursorW(inst voidptr, name voidptr) voidptr
fn C.SetCursor(cursor voidptr) voidptr
fn C.GetKeyState(vk int) i16
fn C.SetCapture(hwnd voidptr) voidptr
fn C.ReleaseCapture() int
fn C.TrackMouseEvent(t &C.TRACKMOUSEEVENT) int
fn C.ValidateRect(hwnd voidptr, r voidptr) int
fn C.ScreenToClient(hwnd voidptr, p &C.POINT) int
fn C.GetLastError() u32
fn C.PostMessageW(hwnd voidptr, msg u32, wparam usize, lparam isize) int
fn C.GetCurrentProcessId() u32

const gwlp_userdata = -21
const cw_usedefault = int(u32(0x80000000))
const ws_overlappedwindow = u32(0x00CF0000)
const cs_hredraw = u32(0x0002)
const cs_vredraw = u32(0x0001)
const idc_arrow = 32512
const pm_remove = u32(1)
const qs_allinput = u32(0x04FF)
const mwmo_inputavailable = u32(0x0004)
const infinite = u32(0xFFFFFFFF)
const swp_nozorder = u32(0x0004)
const swp_noactivate = u32(0x0010)
const tme_leave = u32(0x0002)

const wm_nccreate = u32(0x0081)
const wm_destroy = u32(0x0002)
const wm_quit = u32(0x0012)
const wm_size = u32(0x0005)
const wm_setfocus = u32(0x0007)
const wm_killfocus = u32(0x0008)
const wm_paint = u32(0x000F)
const wm_close = u32(0x0010)
const wm_erasebkgnd = u32(0x0014)
const wm_setcursor = u32(0x0020)
const wm_keydown = u32(0x0100)
const wm_keyup = u32(0x0101)
const wm_char = u32(0x0102)
const wm_syskeydown = u32(0x0104)
const wm_syskeyup = u32(0x0105)
const wm_ime_startcomposition = u32(0x010D)
const wm_ime_endcomposition = u32(0x010E)
const wm_ime_composition = u32(0x010F)
const wm_mousemove = u32(0x0200)
const wm_lbuttondown = u32(0x0201)
const wm_lbuttonup = u32(0x0202)
const wm_rbuttondown = u32(0x0204)
const wm_rbuttonup = u32(0x0205)
const wm_mbuttondown = u32(0x0207)
const wm_mbuttonup = u32(0x0208)
const wm_mousewheel = u32(0x020A)
const wm_xbuttondown = u32(0x020B)
const wm_xbuttonup = u32(0x020C)
const wm_mousehwheel = u32(0x020E)
const wm_entersizemove = u32(0x0231)
const wm_exitsizemove = u32(0x0232)
const wm_ime_setcontext = u32(0x0281)
const wm_mouseleave = u32(0x02A3)
const wm_dpichanged = u32(0x02E0)

@[inline]
fn lo_i16(v isize) int {
	return int(i16(u16(v & 0xFFFF)))
}

@[inline]
fn hi_i16(v isize) int {
	return int(i16(u16((v >> 16) & 0xFFFF)))
}

type FnProcessIdToSessionId = fn (pid u32, session &u32) int

// session_id reports the Windows session of this process (0 = non-interactive services session).
// Resolved at runtime: some C compilers' kernel32 import libraries lack it.
pub fn session_id() u32 {
	kernel32 := load_system_dll('kernel32.dll') or { return max_u32 }
	f := unsafe {
		FnProcessIdToSessionId(dll_proc(kernel32, 'ProcessIdToSessionId') or { return max_u32 })
	}
	mut s := u32(0)
	f(C.GetCurrentProcessId(), &s)
	return s
}
