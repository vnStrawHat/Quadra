// Live resize: frames drawn inside the OS move/size loop (wndproc_windows.c.v).
module win

type FnDwmFlush = fn () int

fn load_dwm_flush() voidptr {
	lib := load_system_dll('dwmapi.dll') or { return unsafe { nil } }
	return dll_proc(lib, 'DwmFlush') or { unsafe { nil } }
}

// after_live_frame waits for the composition that shows the frame just presented, so the
// loop's next window move lands in the same composition as the frame drawn for it.
fn (p &WinPlatform) after_live_frame() {
	if p.dwm_flush != unsafe { nil } {
		flush := unsafe { FnDwmFlush(p.dwm_flush) }
		flush()
	}
}
