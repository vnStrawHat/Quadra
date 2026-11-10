// WinPlatform: the Win32 message loop, timers, cross-thread posts and frame pacing.
module win

import sync
import quadra.platform

fn C.SetTimer(hwnd voidptr, id usize, ms u32, f voidptr) usize
fn C.KillTimer(hwnd voidptr, id usize) int
fn C.QueryPerformanceCounter(out &i64) int
fn C.QueryPerformanceFrequency(out &i64) int

pub type Task = fn ()

const wm_timer = u32(0x0113)
const wm_app = u32(0x8000)
const msg_class_name = 'QuadraMessageWindow'

struct NopSink {}

fn (mut s NopSink) on_event(_ platform.WindowId, _ platform.Event) {}

fn (mut s NopSink) on_frame(_ platform.WindowId) {}

fn (mut s NopSink) on_timer(_ platform.TimerId) {}

fn (mut s NopSink) hit_test(_ platform.WindowId, _ f32, _ f32) platform.HitRegion {
	return .client
}

@[heap]
pub struct WinPlatform {
mut:
	sink       platform.EventSink = NopSink{}
	windows    []&WinWindow
	next_id    u32
	msg_hwnd   voidptr
	posted     []Task
	acts       []PostedAct // screen reader actions, delivered after the UIA call returns
	post_lock  &sync.Mutex = sync.new_mutex()
	timers     map[u64]bool // id -> repeat
	next_timer u64
	quitting   bool
	dpi        Dpi
	shell      Shell32
	ole_       Ole
	ime        Ime
	qpc_freq   i64
	tray_on    bool // tray_windows.c.v
}

pub fn new_platform() !&WinPlatform {
	mut p := &WinPlatform{
		dpi:   load_dpi()!
		shell: load_shell32()
		ime:   load_ime()!
	}
	C.QueryPerformanceFrequency(&p.qpc_freq)
	inst := C.GetModuleHandleW(unsafe { nil })
	wc := C.WNDCLASSEXW{
		cbSize:        u32(sizeof(C.WNDCLASSEXW))
		lpfnWndProc:   voidptr(msg_wnd_proc)
		hInstance:     inst
		lpszClassName: msg_class_name.to_wide()
		lpszMenuName:  unsafe { nil }
	}
	C.RegisterClassExW(&wc)
	// HWND_MESSAGE (-3): an invisible window that only receives posts and timers.
	p.msg_hwnd = C.CreateWindowExW(0, msg_class_name.to_wide(), ''.to_wide(), 0, 0, 0, 0, 0,
		voidptr(-3), unsafe { nil }, inst, voidptr(p))
	if p.msg_hwnd == unsafe { nil } {
		return error('message window creation failed: ${C.GetLastError()}')
	}
	C.SetWindowLongPtrW(p.msg_hwnd, gwlp_userdata, isize(voidptr(p)))
	return p
}

pub fn (mut p WinPlatform) set_sink(mut sink platform.EventSink) {
	p.sink = sink
}

pub fn (mut p WinPlatform) open_window(opts platform.WindowOptions) !platform.PlatformWindow {
	w := p.create_window(opts)!
	p.windows << w
	return w
}

pub fn (p &WinPlatform) is_headless() bool {
	return false
}

pub fn (p &WinPlatform) now_ms() f64 {
	mut t := i64(0)
	C.QueryPerformanceCounter(&t)
	return f64(t) * 1000.0 / f64(p.qpc_freq)
}

pub fn (mut p WinPlatform) quit() {
	p.quitting = true
	C.PostMessageW(p.msg_hwnd, wm_app, 0, 0)
}

pub fn (mut p WinPlatform) post(task fn ()) {
	p.post_lock.@lock()
	p.posted << task
	p.post_lock.unlock()
	C.PostMessageW(p.msg_hwnd, wm_app, 0, 0)
}

pub fn (mut p WinPlatform) set_timer(ms int, repeat bool) platform.TimerId {
	p.next_timer++
	p.timers[p.next_timer] = repeat
	C.SetTimer(p.msg_hwnd, usize(p.next_timer), u32(if ms < 1 { 1 } else { ms }), unsafe { nil })
	return p.next_timer
}

pub fn (mut p WinPlatform) cancel_timer(id platform.TimerId) {
	if id in p.timers {
		p.timers.delete(id)
		C.KillTimer(p.msg_hwnd, usize(id))
	}
}

fn (mut p WinPlatform) run_posted() {
	p.post_lock.@lock()
	tasks := p.posted
	p.posted = []Task{}
	acts := p.acts
	p.acts = []PostedAct{}
	p.post_lock.unlock()
	for t in tasks {
		t()
	}
	for a in acts {
		mut w := a.w
		w.emit(a.ev)
	}
}

struct PostedAct {
	w  &WinWindow = unsafe { nil }
	ev platform.A11yEvent
}

// post_act delivers a screen reader action on the next turn of the message loop, as data:
// no closure per action (V frees none, specs/14).
fn (mut p WinPlatform) post_act(w &WinWindow, ev platform.A11yEvent) {
	p.post_lock.@lock()
	p.acts << PostedAct{w, ev}
	p.post_lock.unlock()
	C.PostMessageW(p.msg_hwnd, wm_app, 0, 0)
}

fn msg_wnd_proc(hwnd voidptr, msg u32, wparam usize, lparam isize) isize {
	ptr := C.GetWindowLongPtrW(hwnd, gwlp_userdata)
	if ptr != 0 {
		mut p := unsafe { &WinPlatform(voidptr(ptr)) }
		if msg == wm_app {
			p.run_posted()
			return 0
		}
		if msg == wm_copydata {
			if e := forwarded_args(lparam) {
				if p.windows.len > 0 {
					p.sink.on_event(p.windows[0].id, e)
				}
				return 1
			}
		}
		if msg == wm_tray {
			if e := tray_event(wparam, lparam) {
				if p.windows.len > 0 {
					p.sink.on_event(p.windows[0].id, e)
				}
			}
			return 0
		}
		if msg == wm_timer {
			id := u64(wparam)
			if repeat := p.timers[id] {
				if !repeat {
					p.cancel_timer(id)
				}
				p.sink.on_timer(id)
			}
			return 0
		}
	}
	return C.DefWindowProcW(hwnd, msg, wparam, lparam)
}

// run pumps messages and paces frames: idle means fully blocked; a requested frame
// waits for its window's swap chain latency object, so it starts as soon as possible.
pub fn (mut p WinPlatform) run() {
	defer {
		p.set_tray('') // no icon left behind in the notification area
	}
	mut msg := C.MSG{}
	for !p.quitting {
		for C.PeekMessageW(&msg, unsafe { nil }, 0, 0, pm_remove) != 0 {
			if msg.message == wm_quit {
				return
			}
			C.TranslateMessage(&msg)
			C.DispatchMessageW(&msg)
		}
		p.windows = p.windows.filter(!it.closed)
		if p.quitting {
			break
		}
		mut waiting := []&WinWindow{}
		mut handles := []voidptr{}
		for mut w in p.windows {
			if !w.frame_requested {
				continue
			}
			if w.waitable == unsafe { nil } {
				w.frame_now()
			} else {
				waiting << w
				handles << w.waitable
			}
		}
		if handles.len == 0 {
			if !p.windows.any(it.frame_requested) {
				C.MsgWaitForMultipleObjectsEx(0, unsafe { nil }, infinite, qs_allinput,
					mwmo_inputavailable)
			}
			continue
		}
		r := C.MsgWaitForMultipleObjectsEx(u32(handles.len), unsafe { &handles[0] }, infinite,
			qs_allinput, mwmo_inputavailable)
		if int(r) < handles.len {
			mut w := waiting[int(r)]
			w.frame_now()
		}
	}
}

pub fn (p &WinPlatform) pty_host() string {
	return conpty_host().name()
}

pub fn (mut p WinPlatform) spawn_pty(opts platform.PtyOptions) !platform.Pty {
	return spawn_conpty(opts)
}
