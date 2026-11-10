// The vertical tab rail (specs/22, Vertical tabs): rows, the collapsed strip and the resize.
module main

import math
import quadra.core
import quadra.kit
import quadra.kit.work
import quadra.prim
import quadra.ui

const rail_collapsed_w = f32(44)

// clamp_tab_rail_width: at least 140, at most max(140, min(400, 35 % of the rail's group)).
pub fn clamp_tab_rail_width(pref f32, group f32) f32 {
	return prim.clamp(pref, f32(rail_width_min), prim.max(f32(rail_width_min), prim.min(f32(rail_width_max),
		group * 0.35)))
}

fn (o &OneTerm) rail_group_width(mut cx core.ViewCx[OneTerm]) f32 {
	win := cx.win.size().width
	if o.dock_open && !is_restricted() {
		return win - clamp_right_dock_width(o.dock_pref, win)
	}
	return win
}

fn (mut o OneTerm) rail_width(mut cx core.ViewCx[OneTerm]) f32 {
	if o.ui.vertical_tabs.collapsed {
		return rail_collapsed_w
	}
	pref := if o.rail_live > 0 { o.rail_live } else { f32(o.ui.vertical_tabs.width) }
	return clamp_tab_rail_width(pref, o.rail_group_width(mut cx))
}

fn (mut o OneTerm) toggle_rail_collapsed(mut _ core.ViewCx[OneTerm]) {
	o.ui.vertical_tabs.collapsed = !o.ui.vertical_tabs.collapsed
	o.persist_ui() or {}
}

// cwd_of is what a tab's second line shows.
fn (o &OneTerm) cwd_of(tab &TermTab, cx &core.Cx) string {
	s := tab.active_space() or { return '' }
	return if s.view.id != 0 { cx.peek(s.view.model()).screen.cwd } else { '' }
}

fn shell_glyph(tab &TermTab) string {
	s := tab.active_space() or { return kit.icon_terminal }
	return match s.shell.kind {
		'cmd' { icon_cmd }
		'powershell', 'pwsh' { icon_powershell }
		else { kit.icon_terminal }
	}
}

fn (mut o OneTerm) rail(t &kit.Theme, mut cx core.ViewCx[OneTerm]) core.Element {
	w := o.rail_width(mut cx)
	collapsed := o.ui.vertical_tabs.collapsed
	plus := o.new_tab_button(.sm)
	mut header := ui.div().flex_row().items_center().h(32).shrink_0()
	if collapsed {
		header.justify_center().px(t.sp(8)).child(plus)
	} else {
		header.pl(t.sp(14)).pr(t.sp(8)).child(ui.text('Terminals').size(t.font_size(.xs)).color(t.p.text_2)).child(ui.div().grow()).child(plus)
	}
	if o.rail_list.y > 0 { // scrolled: a rule keeps the rows apart from the header
		header.child(ui.div().absolute().left(0).right(0).bottom(0).h(1).bg(t.p.line))
	}
	mut list := ui.div().id('rail-list').flex_col().grow().min_h(0).overflow_y_scroll().scroll_with(o.rail_list).on_scroll(cx.listener(fn (mut _ OneTerm, _ &core.ScrollEvent, mut cx core.ViewCx[OneTerm]) {
		cx.notify() // the header rule follows the offset
	}))
	for i, tab in o.tabs {
		list.child(if collapsed {
			o.rail_cell(i, tab, t, mut cx)
		} else {
			o.rail_row(i, tab, t, mut cx)
		})
	}
	list.child(ui.div().grow().min_h(16).test_id('rail-end').on_drop(work.tab_drag_kind, cx.listener(fn (mut o OneTerm, e &core.DropEvent, mut cx core.ViewCx[OneTerm]) {
		o.move_tab(work.tab_drag_id(e.payload), o.tabs.len - 1)
		cx.notify()
	})))
	mut box :=
		ui.div().test_id('rail').flex_col().w(w).h_full().shrink_0().bg(t.p.tab_bg).border_r(1).border_color(t.p.line_strong).child(header).child(list)
	if !collapsed {
		// Over the rail's 1 px border; filled with the focus colour on hover and while dragged.
		mut handle := ui.div().id('rail-handle').test_id('rail-handle').absolute().top(0).bottom(0).right(0).w(4).cursor(.resize_ew).hover(ui.Patch{
			background: t.p.border_focused
		})
		if o.rail_drag {
			handle.bg(t.p.border_focused)
		}
		box.child(handle.on_mouse_down(core.handler(fn (_ &core.MouseDownEvent, mut cx core.Cx) {
			cx.capture_mouse()
		})).on_mouse_move(cx.listener(fn (mut o OneTerm, e &core.MouseMoveEvent, mut cx core.ViewCx[OneTerm]) {
			if e.pressed {
				o.rail_drag = true
				o.rail_live = prim.clamp(e.x, f32(rail_width_min), f32(rail_width_max))
				cx.notify()
			}
		})).on_mouse_up(cx.listener(fn (mut o OneTerm, _ &core.MouseUpEvent, mut cx core.ViewCx[OneTerm]) {
			if o.rail_live > 0 && int(o.rail_live) != o.ui.vertical_tabs.width {
				o.ui.vertical_tabs.width = int(o.rail_live)
				o.persist_ui() or {}
			}
			o.rail_live = 0
			o.rail_drag = false
			cx.notify()
		})))
	}
	return box
}

// RowEvent names a rail row for its handlers.
struct RowEvent {
	id    string
	index int
}

fn (o &OneTerm) rail_events(d &ui.Div, r RowEvent, title string, glyph string, mut cx core.ViewCx[OneTerm]) &ui.Div {
	return d.drag_source(work.tab_drag_source(r.id, title, glyph)).on_drop(work.tab_drag_kind, core.listener_with[OneTerm, int, core.DropEvent](cx.entity,
		r.index, fn (mut o OneTerm, at int, e &core.DropEvent, mut cx core.ViewCx[OneTerm]) {
		o.move_tab(work.tab_drag_id(e.payload), at)
		cx.notify()
	})).on_mouse_down(core.listener_with[OneTerm, string, core.MouseDownEvent](cx.entity, r.id, fn (mut o OneTerm, id string, e &core.MouseDownEvent, mut cx core.ViewCx[OneTerm]) {
		match e.button {
			.middle {
				o.close_tab(id, mut cx)
			}
			.right {
				kit.open_context_menu(mut cx.Cx, e.x, e.y, o.tab_menu(id, true))
			}
			.left {
				if e.clicks == 2 && !o.ui.vertical_tabs.collapsed {
					o.ask_rename(id, mut cx)
				}
			}
			else {}
		}
	})).on_click(core.listener_with[OneTerm, string, core.ClickEvent](cx.entity, r.id, fn (mut o OneTerm, id string, _ &core.ClickEvent, mut cx core.ViewCx[OneTerm]) {
		o.select_tab(id, mut cx) // on release: a press may start a drag out of this row
	}))
}

// rail_row is a 48 px, two-line row; the selected one has a 3 px accent pill.
fn (mut o OneTerm) rail_row(i int, tab &TermTab, t &kit.Theme, mut cx core.ViewCx[OneTerm]) core.Element {
	on := tab.id == o.active
	title := o.title_of(tab, &cx.Cx)
	cwd := o.cwd_of(tab, &cx.Cx)
	status := if tab.spaces.len == 1 {
		if s := tab.active_space() { status_icon(program_status(s), t) } else { none }
	} else {
		none
	}
	// The selected row's fill is too dark for text_2 in some light themes: halfway to the title.
	cwd_color := if on { t.p.text_2.lerp(t.p.tab_active_fg, 0.5) } else { t.p.text_2 }
	mut line1 :=
		ui.div().flex_row().items_center().gap(4).h(20).children(o.tab_lead_no_status(tab, t)).child(ui.text(title).truncate())
	mut text_col := ui.div().flex_col().grow().min_w(0).justify_center().child(line1)
	if cwd != '' {
		mut line2 :=
			ui.div().flex_row().items_center().h(16).child(ui.div().grow().min_w(0).child(ui.text(trim_start(cwd, 32)).size(t.font_size(.xs)).color(cwd_color)))
		if st := status {
			line2.child(st)
		}
		text_col.child(line2)
	} else if st := status {
		line1.child(ui.div().grow()).child(st)
	}
	fill := if on { t.p.text.alpha(0.10) } else { prim.transparent }
	mut row := ui.div().id('rail-${tab.id}').test_id('rail-${tab.id}').flex_row().items_center().h(48).shrink_0().pl(14).pr(10).bg(fill).text_color(if on {
		t.p.tab_active_fg
	} else {
		t.p.tab_fg
	}).cursor(.pointer).role(.tab).label(title).selected(on).child(text_col)
	if !on { // the title takes the active colour under the pointer, as a tab does
		row.hover(ui.Patch{
			background: t.p.text.alpha(0.05)
			text_color: t.p.tab_active_fg
		})
	} else {
		row.child(ui.div().absolute().left(0).top(8).bottom(8).w(3).rounded(2).bg(t.p.accent))
	}
	return o.rail_events(row, RowEvent{tab.id, i}, title, shell_glyph(tab), mut cx)
}

// rail_cell is a collapsed tab: the shell's glyph, the OSC 7501 state as the fill.
fn (mut o OneTerm) rail_cell(i int, tab &TermTab, t &kit.Theme, mut cx core.ViewCx[OneTerm]) core.Element {
	on := tab.id == o.active
	title := o.title_of(tab, &cx.Cx)
	cwd := o.cwd_of(tab, &cx.Cx)
	mut fill := if on { t.p.text.alpha(0.10) } else { prim.transparent }
	mut glyph := if on { t.p.tab_active_fg } else { t.p.tab_fg }
	mut status := false
	if tab.spaces.len == 1 {
		if s := tab.active_space() {
			match program_status(s) {
				.working { fill = t.p.accent }
				.blocked { fill = t.p.warn }
				.done { fill = t.p.ok }
				.error { fill = t.p.error }
				.idle {}
			}

			status = program_status(s) != .idle
		}
	}
	if status { // light on a dark fill, dark on a light one
		glyph = legible_on(fill, t.p.canvas, t.p.text)
	}
	mut cell :=
		ui.div().id('rail-${tab.id}').test_id('rail-${tab.id}').size(44).shrink_0().center().bg(fill).cursor(.pointer).role(.tab).label(title).selected(on).child(kit.icon(shell_glyph(tab)).size(t.sp(16)).color(glyph))
	if on { // over a status fill the pill takes the glyph's colour, which reads on any fill
		pill := if status { glyph } else { t.p.accent }
		cell.child(ui.div().absolute().left(0).top(8).bottom(8).w(3).rounded(2).bg(pill))
	}
	if s := tab.active_space() {
		if is_recording(s) {
			dot := if status { glyph } else { t.p.error }
			cell.child(ui.div().absolute().top(3).left(3).child(recording_dot(dot, t)))
		}
	}
	return kit.hover_card_with(o.rail_events(cell, RowEvent{tab.id, i}, title, shell_glyph(tab), mut
		cx), RailCard{title, cwd}, rail_card)
}

struct RailCard {
	title string
	cwd   string
}

fn rail_card(c RailCard, t &kit.Theme) core.Element {
	mut col := ui.div().flex_col().gap(2).max_w(280).child(ui.text(c.title))
	if c.cwd != '' {
		col.child(ui.text(trim_start(c.cwd, 40)).color(t.p.text_2))
	}
	return col
}

// tab_lead_no_status is the lead of a rail row, whose status icon goes on the second line.
fn (o &OneTerm) tab_lead_no_status(tab &TermTab, t &kit.Theme) []core.Element {
	mut out := []core.Element{}
	for c in channels {
		if tab.spaces.any(it.channel == c) {
			out << channel_chip(c, t)
		}
	}
	if s := tab.active_space() {
		if is_recording(s) {
			out << recording_dot(t.p.error, t)
		}
	}
	return out
}

// recording_dot is the `●` glyph at the xsmall text size.
fn recording_dot(c prim.Color, t &kit.Theme) core.Element {
	return ui.text('●').size(t.font_size(.xs)).color(c)
}

// legible_on is whichever of a and b has the higher WCAG contrast on fill (a wins a tie).
fn legible_on(fill prim.Color, a prim.Color, b prim.Color) prim.Color {
	return if contrast(b, fill) > contrast(a, fill) { b } else { a }
}

fn contrast(x prim.Color, y prim.Color) f32 {
	lx, ly := luminance(x) + 0.05, luminance(y) + 0.05
	return if lx > ly { lx / ly } else { ly / lx }
}

fn channel_lin(v f32) f32 {
	return if v <= 0.03928 { v / 12.92 } else { f32(math.pow((v + 0.055) / 1.055, 2.4)) }
}

fn luminance(c prim.Color) f32 {
	return 0.2126 * channel_lin(c.r) + 0.7152 * channel_lin(c.g) + 0.0722 * channel_lin(c.b)
}

// trim_start cuts a long path at its start.
fn trim_start(s string, keep int) string {
	r := s.runes()
	return if r.len <= keep { s } else { '…' + r[r.len - keep..].string() }
}
