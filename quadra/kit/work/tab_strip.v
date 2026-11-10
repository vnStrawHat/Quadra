// The tab strip of a tab group (32 px): icon, title, a dirty dot or close button; middle
// click closes, drag reorders, and a menu lists every tab when they overflow.
module work

import math
import quadra.core
import quadra.prim
import quadra.layout
import quadra.kit
import quadra.ui

pub struct Tab {
pub:
	id    string
	title string
	glyph string
	dirty bool
}

// TabMove asks to move a tab to index `to` (the app reorders its list).
pub struct TabMove {
pub:
	id string
	to int
}

pub struct DragState {
mut:
	id    string
	x0    f32
	moved bool
	rects []prim.Rect // tab bounds from the last prepaint
}

@[heap]
pub struct TabStrip {
	kit.Lazy
pub mut:
	id      string
	tabs    []Tab
	active  string
	actions []core.Element
	after   []core.Element            // after the overflow menu
	leads   map[string][]core.Element // by tab id: drawn before the title (status, chips)
	min_w   f32                       // the tab content's width limits; 0: none
	max_w   f32
	change  ?core.AnyListener
	close   ?core.AnyListener
	move    ?core.AnyListener
	heading string            // the `...` list's heading; set, the list also offers Close
	menu    ?core.AnyListener // right click: a TabPoint
	rename  ?core.AnyListener // double click: the tab's id
	// docking (dock.v): this strip's group, and the listener of tabs dropped on it
	dock_group string
	dock       ?core.AnyListener
mut:
	st    &DragState = unsafe { nil }
	rects []prim.Rect
}

pub fn tab_strip(id string, tabs []Tab, active string) &TabStrip {
	return &TabStrip{
		id:     id
		tabs:   tabs
		active: active
	}
}

pub fn (self_ &TabStrip) on_select(l core.Listener[string]) &TabStrip {
	mut s := unsafe { self_ }
	s.change = l.any
	return s
}

pub fn (self_ &TabStrip) on_close(l core.Listener[string]) &TabStrip {
	mut s := unsafe { self_ }
	s.close = l.any
	return s
}

pub fn (self_ &TabStrip) on_move(l core.Listener[TabMove]) &TabStrip {
	mut s := unsafe { self_ }
	s.move = l.any
	return s
}

// dock lets tabs be torn off (dragged 12 px up or down) and dropped on other strips and
// panes, and accepts tabs dropped here; group names this strip's tab group.
pub fn (self_ &TabStrip) dock(group string, l core.Listener[DockEvent]) &TabStrip {
	mut s := unsafe { self_ }
	s.dock_group = group
	s.dock = l.any
	return s
}

// actions adds elements at the right end (split, more, ...).
pub fn (self_ &TabStrip) actions(els []core.Element) &TabStrip {
	mut s := unsafe { self_ }
	s.actions = els
	return s
}

pub fn (mut s TabStrip) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut s, mut cx)
	return s.layout_inner(el, mut cx)
}

pub fn (mut s TabStrip) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.enter('tabstrip:${s.id}')
	s.st = cx.state(DragState{})
	cx.leave()
	s.Lazy.prepaint(bounds, mut cx)
	s.st.rects = s.rects.clone()
}

pub fn (mut s TabStrip) build_tree(mut cx core.LayoutCx) core.Element {
	t := kit.theme_of(cx.app)
	// Probes write into s.rects: fixed length, never appended; a rebuild makes a new array
	// together with the tree that points into it.
	s.rects = []prim.Rect{len: s.tabs.len}
	mut row :=
		ui.div().flex_row().h_full().grow().overflow_x_scroll().wheel_x().role(.tablist).text_size(t.ui_text(.md))
	for i, tab in s.tabs {
		row.child(Probe{
			el:  s.tab(i, tab, t)
			out: unsafe { &s.rects[i] }
		})
	}
	items := s.list_items()
	// The bar's bottom line runs under the inactive tabs, the empty space and the actions;
	// the active tab has none, so it joins its content (specs/15).
	row.child(ui.div().grow().h_full().border_b(1).border_color(t.p.line_strong))
	more := kit.menu_button('${s.id}-more', '', items).icon(kit.icon_ellipsis).ghost().size(.sm)
	return s.drop_target(ui.div().id(s.id).test_id(s.id).flex_row().h(t.tabs_h).shrink_0().bg(t.p.tab_bg).child(row).child(ui.div().flex_row().items_center().h_full().gap(4).px(6).border_b(1).border_l(1).border_color(t.p.line_strong).children(s.actions).child(more).children(s.after)))
}

// tab (specs/15): 32 tall, the active one on the canvas tone with no bottom line; seams on
// the side away from the active tab; no hover fill. The dirty dot is always shown, the
// close button on hover.
fn (s &TabStrip) tab(i int, tab Tab, t &kit.Theme) core.Element {
	if t.ctl.accent_chrome {
		return s.tab_accent(i, tab, t)
	}
	p := t.p
	on := tab.id == s.active
	at := s.tabs.map(it.id).index(s.active)
	id := tab.id
	strip := unsafe { s }
	dot := if tab.dirty {
		core.Element(ui.div().size(6).rounded(3).bg(p.accent))
	} else {
		core.Element(ui.div())
	}
	mut content :=
		ui.div().flex_row().items_center().gap(4).px(4).grow().child(ui.div().size(12).center().child(dot))
	if tab.glyph != '' {
		content.child(kit.icon(tab.glyph).size(14).color(p.text_2))
	}
	content.children(s.leads[tab.id])
	content.child(ui.text(tab.title).truncate().color(if on { p.tab_active_fg } else { p.tab_fg }))
	if strip.close != none { // tabs are closable only when someone listens
		// No color of its own: the icon takes the tab's text color, the tab's own fill
		// until the pointer is over the tab (the hover patch below).
		content.child(ui.div().size(14).center().child(ui.div().test_id('${s.id}-${tab.id}-close').size(18).shrink_0().center().rounded(t.radius_md).hover(ui.Patch{
			background: p.panel_hover
		}).child(kit.icon(kit.icon_x).size(14)).on_mouse_down(core.handler(fn (_ &core.MouseDownEvent, mut cx core.Cx) {
			cx.stop_propagation() // pressing close must not select or start a drag
		})).on_click(core.handler_with(StripTab{strip, id}, fn (st StripTab, _ &core.ClickEvent, mut cx core.Cx) {
			cx.stop_propagation()
			kit.fire(st.strip.close, st.id, mut cx)
		}))))
	} else {
		content.pr(8)
	}
	if s.min_w > 0 {
		content.min_w(s.min_w).max_w(s.max_w)
	}
	fill := if on { p.canvas } else { p.tab_bg }
	mut d := ui.div().id('tab:${tab.id}').test_id('${s.id}-${tab.id}').flex_row().h_full().shrink_0().cursor(.pointer).border_color(p.line_strong).bg(fill).text_color(fill).hover(ui.Patch{
		text_color: p.text // the close button: full text colour, a muted one read as disabled
	}).role(.tab).label(tab.title).selected(on).child(content)
	if !on {
		d.border_b(1)
	}
	if i > 0 && (i <= at || at < 0) {
		d.border_l(1)
	}
	if i >= at && at >= 0 {
		d.border_r(1)
	}
	return s.tab_events(d, tab)
}

// tab_events makes a tab draggable and wires its clicks.
fn (s &TabStrip) tab_events(d &ui.Div, tab Tab) core.Element {
	strip := unsafe { s }
	id := tab.id
	if s.dock != none {
		d.drag_source(core.DragSource{
			payload:    dock_payload(s.dock_group, tab.id)
			preview_el: core.Element(&TabChip{
				title: tab.title
				glyph: tab.glyph
			})
			tear_off:   12
		})
	}
	d.on_mouse_down(core.handler_with(StripTab{strip, id}, fn (sd StripTab, e &core.MouseDownEvent, mut cx core.Cx) {
		strip, id := sd.strip, sd.id
		if e.button == .middle {
			kit.fire(strip.close, id, mut cx)
			return
		}
		if e.button == .right {
			kit.fire(strip.menu, TabPoint{id, e.x, e.y}, mut cx)
			return
		}
		if e.button != .left || strip.st == unsafe { nil } {
			return
		}
		if e.clicks == 2 {
			kit.fire(strip.rename, id, mut cx)
		}
		mut st := strip.st
		st.id, st.x0, st.moved = id, e.x, false
		cx.capture_mouse()
		if strip.dock == none { // a draggable tab activates on release: it may be dropped elsewhere
			kit.fire(strip.change, id, mut cx)
		}
	}))
	d.on_mouse_up(core.handler_with(StripTab{strip, id}, fn (sd StripTab, _ &core.MouseUpEvent, mut cx core.Cx) {
		if sd.strip.dock != none && sd.strip.st != unsafe { nil } && sd.strip.st.id == sd.id
			&& !sd.strip.st.moved {
			kit.fire(sd.strip.change, sd.id, mut cx)
		}
	}))
	d.on_mouse_move(core.handler_with(StripTab{strip, ''}, fn (sd StripTab, e &core.MouseMoveEvent, mut cx core.Cx) {
		sd.strip.drag(e, mut cx)
	}))
	return d
}

// drag moves the held tab to the slot under the mouse once it has moved 4 px.
fn (s &TabStrip) drag(e &core.MouseMoveEvent, mut cx core.Cx) {
	if s.st == unsafe { nil } || s.st.id == '' {
		return
	}
	mut st := s.st
	if !e.pressed {
		st.id = ''
		return
	}
	if !st.moved && math.abs(e.x - st.x0) < 4 {
		return
	}
	st.moved = true
	from := s.tabs.map(it.id).index(st.id)
	mut to := from
	for i, r in st.rects {
		if e.x >= r.x && e.x < r.x + r.width {
			to = i
		}
	}
	if from >= 0 && to != from {
		kit.fire(s.move, TabMove{st.id, to}, mut cx)
	}
}

// StripTab is a tab's handlers' data: its strip and id.
struct StripTab {
	strip &TabStrip = unsafe { nil }
	id    string
}

// SelectTab is the overflow menu's action for one tab.
struct SelectTab {
	change ?core.AnyListener
	id     string
}

pub fn (s SelectTab) run(mut cx core.Cx) {
	kit.fire(s.change, s.id, mut cx)
}
