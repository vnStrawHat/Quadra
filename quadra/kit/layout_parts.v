// Layout parts: scroll_area (styled scrollbar) and split (resizable panes).
module kit

import quadra.core
import quadra.prim
import quadra.layout
import quadra.platform
import quadra.ui

@[heap]
pub struct ScrollArea {
pub mut:
	content core.Element
	height  f32
mut:
	div &ui.Div = unsafe { nil }
}

// scroll_area scrolls its content vertically and draws a thin scrollbar.
pub fn scroll_area(content core.Element, height f32) &ScrollArea {
	return &ScrollArea{
		content: content
		height:  height
	}
}

pub fn (mut s ScrollArea) request_layout(mut cx core.LayoutCx) layout.NodeId {
	s.div = ui.div().flex_col().overflow_y_scroll().child(s.content)
	if s.height > 0 {
		s.div.h(s.height)
	} else {
		s.div.size_full()
	}
	return s.div.request_layout(mut cx)
}

pub fn (mut s ScrollArea) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	s.div.prepaint(bounds, mut cx)
}

pub fn (mut s ScrollArea) paint(bounds prim.Rect, mut cx core.PaintCx) {
	s.div.paint(bounds, mut cx) // the Div draws its own auto-hiding thumb
}

pub struct SplitState {
pub mut:
	bounds prim.Rect
}

@[heap]
pub struct Split {
pub mut:
	id       string
	first    core.Element
	second   core.Element
	ratio    f32 = 0.3
	vertical bool // stack top/bottom instead of left/right
	min_px   f32 = 80
	resize   ?core.AnyListener
mut:
	div   &ui.Div     = unsafe { nil }
	state &SplitState = unsafe { nil }
}

// split shows two panes with a draggable divider; the parent owns the ratio.
pub fn split(id string, first core.Element, second core.Element, ratio f32) &Split {
	return &Split{
		id:     id
		first:  first
		second: second
		ratio:  ratio
	}
}

pub fn (self_ &Split) vertical() &Split {
	mut s := unsafe { self_ }
	s.vertical = true
	return s
}

pub fn (self_ &Split) on_resize(l core.Listener[f32]) &Split {
	mut s := unsafe { self_ }
	s.resize = l.any
	return s
}

fn split_down(holder prim.Holder, _ prim.Holder, event prim.Holder, mut cx core.Cx) {
	s := prim.held[Split](holder)
	e := prim.held[core.MouseDownEvent](event)
	if e.clicks == 2 {
		fire(s.resize, f32(0.5), mut cx) // double-click resets to an even split
		return
	}
	cx.capture_mouse()
}

fn split_drag(holder prim.Holder, _ prim.Holder, event prim.Holder, mut cx core.Cx) {
	s := prim.held[Split](holder)
	e := prim.held[core.MouseMoveEvent](event)
	if !e.pressed || s.state == unsafe { nil } {
		return
	}
	b := s.state.bounds
	total := if s.vertical { b.height } else { b.width }
	pos := if s.vertical { e.y - b.y } else { e.x - b.x }
	if total <= 0 {
		return
	}
	r := prim.clamp(pos, s.min_px, total - s.min_px) / total
	fire(s.resize, r, mut cx)
}

pub fn (mut s Split) request_layout(mut cx core.LayoutCx) layout.NodeId {
	t := theme_of(cx.app)
	mut a := ui.div().overflow_hidden().child(s.first)
	mut b := ui.div().overflow_hidden().grow().child(s.second)
	if s.vertical {
		a.h_len(layout.pct(s.ratio * 100)).w_full()
		b.w_full()
	} else {
		a.w_len(layout.pct(s.ratio * 100)).h_full()
		b.h_full()
	}
	cursor := if s.vertical { platform.Cursor.resize_ns } else { platform.Cursor.resize_ew }
	// A 1 px seam in a 6 px grab area; no hover mark, or a pill with accent_chrome (specs/15).
	chrome := t.ctl.accent_chrome
	line := resize_line(s.vertical, if chrome { t.p.line } else { t.p.line_strong }, t)
	mut handle := ui.div().test_id('${s.id}-handle').center().cursor(cursor).child(line)
	if chrome {
		handle.group()
	}

	// Handlers are holder + function, not closures: layout runs every frame (specs/14).
	handle.handlers << ui.Handler{.mouse_down, false, core.AnyListener{
		holder: prim.hold(s)
		call:   split_down
	}}
	handle.handlers << ui.Handler{.mouse_move, false, core.AnyListener{
		holder: prim.hold(s)
		call:   split_drag
	}}
	if s.vertical {
		handle.h(6).w_full().shrink_0()
	} else {
		handle.w(6).h_full().shrink_0()
	}
	s.div = ui.div().size_full().child(a).child(handle).child(b)
	if s.vertical {
		s.div.flex_col()
	} else {
		s.div.flex_row()
	}
	return s.div.request_layout(mut cx)
}

pub fn (mut s Split) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.enter('split:${s.id}')
	s.state = cx.state(SplitState{})
	s.state.bounds = bounds
	cx.leave()
	s.div.prepaint(bounds, mut cx)
}

pub fn (mut s Split) paint(bounds prim.Rect, mut cx core.PaintCx) {
	s.div.paint(bounds, mut cx)
}

// resize_line is a resize handle's 1 px hairline (`vertical`: across stacked panes). With
// Controls.accent_chrome it carries the handle's pill (specs/15): 3 px in `text_2`, 20 long
// at 35 % while the handle, a ui group(), is hovered and 44 long at 90 % while it is pressed.
pub fn resize_line(vertical bool, color prim.Color, t &Theme) &ui.Div {
	mut line := ui.div().bg(color).shrink_0()
	if vertical {
		line.h(1).w_full()
	} else {
		line.w(1).h_full()
	}
	if !t.ctl.accent_chrome {
		return line
	}
	// ponytail: hover and press only; the original's 28 px pressed-but-still step and its
	// length easing are left out (a pill element with its own state could add them)
	clear := prim.transparent
	mut short := ui.div().shrink_0().bg(clear).rounded_full().group_hover(ui.Patch{
		background: t.p.text_2.alpha(0.35)
	}).group_active(ui.Patch{
		background: clear
	})
	mut long := ui.div().shrink_0().center().bg(clear).rounded_full().group_active(ui.Patch{
		background: t.p.text_2.alpha(0.9)
	}).child(short)
	// The pill is centred along the line and overhangs it by 1 on each side.
	if vertical {
		line.flex_row().justify_center()
		long.w(44).h(3).my(-1)
		short.w(20).h(3)
	} else {
		line.flex_col().justify_center()
		long.w(3).h(44).mx(-1)
		short.w(3).h(20)
	}
	return line.child(long)
}
