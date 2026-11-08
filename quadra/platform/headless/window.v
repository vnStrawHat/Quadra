module headless

import quadra.prim
import quadra.platform

@[heap]
pub struct HeadlessWindow {
pub:
	id platform.WindowId
pub mut:
	title           string
	sz              prim.Size
	scale_          f32 = 1
	cursor          platform.Cursor
	ime_rect        prim.Rect
	ime_enabled     bool = true
	frame_requested bool
	closed          bool
	place           platform.Placement
	maximized       bool
	minimized       bool
	a11y_on         bool
	a11y            []platform.A11yNode
}

pub fn (w &HeadlessWindow) is_maximized() bool {
	return w.maximized
}

pub fn (mut w HeadlessWindow) minimize() {
	w.minimized = true
}

pub fn (mut w HeadlessWindow) toggle_maximize() {
	w.maximized = !w.maximized
}

pub fn (w &HeadlessWindow) placement() platform.Placement {
	return platform.Placement{
		...w.place
		width:  int(w.sz.width)
		height: int(w.sz.height)
		valid:  true
	}
}

pub fn (w &HeadlessWindow) size() prim.Size {
	return w.sz
}

pub fn (w &HeadlessWindow) scale() f32 {
	return w.scale_
}

pub fn (w &HeadlessWindow) native_handle() voidptr {
	return unsafe { nil }
}

pub fn (mut w HeadlessWindow) set_title(title string) {
	w.title = title
}

pub fn (mut w HeadlessWindow) set_cursor(c platform.Cursor) {
	w.cursor = c
}

pub fn (mut w HeadlessWindow) request_frame() {
	w.frame_requested = true
}

pub fn (mut w HeadlessWindow) set_ime_rect(r prim.Rect) {
	w.ime_rect = r
}

pub fn (mut w HeadlessWindow) enable_ime(on bool) {
	w.ime_enabled = on
}

pub fn (mut w HeadlessWindow) set_waitable(_ voidptr) {}

pub fn (mut w HeadlessWindow) close() {
	w.closed = true
}

pub fn (w &HeadlessWindow) is_closed() bool {
	return w.closed
}

// Tests turn a11y on to see the tree the app would send.
pub fn (w &HeadlessWindow) a11y_active() bool {
	return w.a11y_on
}

pub fn (mut w HeadlessWindow) update_a11y(nodes []platform.A11yNode) {
	w.a11y = nodes
}
