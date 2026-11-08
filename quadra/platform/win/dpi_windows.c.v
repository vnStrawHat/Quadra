// Per-monitor DPI v2. These entry points are newer than the import libraries
// some C compilers ship, so they are resolved at runtime.
module win

type FnSetProcessDpiAwarenessContext = fn (ctx voidptr) int

type FnGetDpiForWindow = fn (hwnd voidptr) u32

type FnAdjustWindowRectExForDpi = fn (r &C.RECT, style u32, menu int, ex u32, dpi u32) int

struct Dpi {
mut:
	get_dpi_for_window FnGetDpiForWindow          = unsafe { nil }
	adjust_rect        FnAdjustWindowRectExForDpi = unsafe { nil }
}

const dpi_awareness_per_monitor_v2 = voidptr(-4)

fn load_dpi() !Dpi {
	user32 := load_system_dll('user32.dll')!
	set_ctx := unsafe {
		FnSetProcessDpiAwarenessContext(dll_proc(user32, 'SetProcessDpiAwarenessContext')!)
	}
	set_ctx(dpi_awareness_per_monitor_v2)
	return Dpi{
		get_dpi_for_window: unsafe { FnGetDpiForWindow(dll_proc(user32, 'GetDpiForWindow')!) }
		adjust_rect:        unsafe {
			FnAdjustWindowRectExForDpi(dll_proc(user32, 'AdjustWindowRectExForDpi')!)
		}
	}
}

fn (d &Dpi) scale_of(hwnd voidptr) f32 {
	return f32(d.get_dpi_for_window(hwnd)) / 96.0
}
