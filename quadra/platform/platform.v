// OS boundary: everything core needs from the operating system (specs/02-platform.md).
module platform

import quadra.prim

pub type TimerId = u64

pub enum Cursor {
	arrow
	ibeam
	pointer
	resize_ew
	resize_ns
	resize_nwse
	resize_nesw
	move
	not_allowed
	wait
	crosshair
	hidden
}

// HitRegion is what a point of a custom titlebar does for the OS: drag the window, or act
// as one of the window buttons (Windows 11 shows snap layouts over `maximize`).
pub enum HitRegion {
	client
	caption
	minimize
	maximize
	close
}

pub enum WindowKind {
	normal
	popup
}

// Placement is a window's restored frame in screen device px, plus its maximized state.
// It is what window-state persistence saves and restores.
pub struct Placement {
pub:
	x         int
	y         int
	width     int
	height    int
	maximized bool
	valid     bool // false: let the OS choose
}

pub struct WindowOptions {
pub:
	placement Placement // restored when valid and still on a connected monitor
	// custom_titlebar: the app draws the titlebar (kit.titlebar); the OS keeps resizing,
	// snapping and the window buttons' behaviour through EventSink.hit_test.
	custom_titlebar bool
	title           string = 'Quadra'
	width           f32    = 960 // logical px
	height          f32    = 600
	min_width       f32
	min_height      f32
	resizable       bool = true
	kind            WindowKind
	parent          WindowId
	// centered opens the window centred on the primary display (else where the OS puts it);
	// fit, when above 0, caps each side at that fraction of the display first.
	centered bool
	fit      f32
}

// fit_size is a window size capped by WindowOptions.fit against a display (logical px).
pub fn fit_size(w f32, h f32, display prim.Size, fit f32) prim.Size {
	mut s := prim.size(w, h)
	if fit > 0 && display.width > 0 && display.height > 0 {
		if s.width > display.width * fit {
			s.width = display.width * fit
		}
		if s.height > display.height * fit {
			s.height = display.height * fit
		}
	}
	return s
}

pub interface PlatformWindow {
	id WindowId
	size() prim.Size // logical px
	scale() f32      // 1.0 = 96 DPI
	native_handle() voidptr
	is_closed() bool
	placement() Placement
	is_maximized() bool
mut:
	minimize()
	toggle_maximize()
	set_title(title string)
	set_cursor(c Cursor)
	request_frame()
	set_ime_rect(r prim.Rect) // logical px
	enable_ime(on bool)
	set_waitable(handle voidptr) // swap chain frame-latency object
	// a11y_active: assistive technology has asked for this window's tree since it opened;
	// only then does the app send it (update_a11y, after each prepaint).
	a11y_active() bool
	update_a11y(nodes []A11yNode)
	close()
}

// Appearance is the system look the app may follow (kit.follow_system).
pub struct Appearance {
pub:
	dark          bool
	accent        prim.Color = prim.rgb(0x2563EB)
	high_contrast bool
	text_scale    f32 = 1
}

pub interface Platform {
	now_ms() f64 // monotonic
	appearance() Appearance
	clipboard_read() ?string
	is_headless() bool
	// pty_host names the pseudo terminal host spawn_pty uses ('conpty: bundled', 'conpty:
	// system'; '' without one), for the log (specs/17).
	pty_host() string
mut:
	set_sink(mut sink EventSink)
	run() // blocks until quit or the last window closes
	quit()
	open_window(opts WindowOptions) !PlatformWindow
	post(task fn ())                       // any thread; runs on the main thread
	set_timer(ms int, repeat bool) TimerId // fires sink.on_timer
	cancel_timer(id TimerId)
	clipboard_write(s string)
	// claim_single_instance: true for the first instance of key; later instances forward
	// args to it (delivered as InstanceArgs) and get false.
	claim_single_instance(key string, args []string) bool
	// install_crash_handler writes a minidump and extra() (recent log lines) to dir when
	// the process dies of an unhandled exception.
	install_crash_handler(dir string, extra fn () string)
	// spawn_pty starts a program on a pseudo terminal (specs/17); headless has none.
	spawn_pty(opts PtyOptions) !Pty
	// pick_files shows a modal open or save dialog over owner (0: none) and returns the
	// chosen paths, empty when cancelled. Headless returns what a test queued.
	pick_files(opts FileDialog, owner WindowId) []string
	// open_external opens a URL or a file with its default app; reveal shows a file in the
	// file manager. Both report whether the shell accepted the request.
	open_external(target string) bool
	reveal(path string) bool
	// message_box shows a modal native message over owner (0: none) and returns the button
	// pressed. Headless returns what a test queued.
	message_box(m MessageBox, owner WindowId) MessageResult
	// set_tray shows the app's icon in the notification area with a tooltip ('' removes it);
	// clicks arrive as TrayEvent at the first window. notify shows a system notification
	// from it. Both report whether the shell accepted.
	set_tray(tooltip string) bool
	notify(title string, body string) bool
}

// FileDialog describes an open or save dialog (Platform.pick_files).
pub struct FileDialog {
pub:
	title        string
	save         bool // a save dialog: one path, which may not exist yet
	folders      bool // pick folders instead of files
	multiple     bool // open several files
	filters      []FileFilter
	default_name string // the suggested file name
	dir          string // the folder to start in
}

// FileFilter is one entry of a dialog's type list: a name and patterns such as '*.yaml;*.yml'.
pub struct FileFilter {
pub:
	name     string
	patterns string
}

// Pty is a running program on a pseudo terminal. Output arrives through PtyOptions.on_data
// on a reader thread.
// What a PTY or SSH shell queues for its writer thread; a write past either fails and is dropped.
pub const pty_write_budget = 4 << 20
pub const pty_write_items = 256

pub interface Pty {
mut:
	write(data []u8) !
	resize(cols int, rows int) !
	close()
}

pub struct PtyOptions {
pub:
	command string // the command line; empty = the user's shell
	cwd     string
	env     map[string]string // added to the parent's environment; an empty value removes the variable
	cols    int = 80
	rows    int = 24
	// ctx is handed back to on_data and on_exit: plain functions with their data, no closure
	// per terminal (V frees none, specs/14)
	ctx     prim.Holder
	on_data fn (ctx prim.Holder, data []u8) = unsafe { nil } // reader thread; data is lent for the call: copy to keep it
	on_exit fn (ctx prim.Holder, code int)  = unsafe { nil } // reader thread, once, after the last data
}

// merged_env is the `parent` environment with `extra` applied, as sorted KEY=value lines (names
// compare case-insensitively, as on Windows); an empty value in extra removes the variable.
pub fn merged_env(parent map[string]string, extra map[string]string) []string {
	mut by_name := map[string]string{}
	for k, v in parent {
		by_name[k.to_upper()] = '${k}=${v}'
	}
	for k, v in extra {
		if v == '' {
			by_name.delete(k.to_upper())
		} else {
			by_name[k.to_upper()] = '${k}=${v}'
		}
	}
	mut names := by_name.keys()
	names.sort()
	return names.map(by_name[it])
}
