// LogView: an append-only log with ANSI colors, follow mode and a filter (specs/19). The
// LogSurface element paints the visible lines (log_paint.v).
module data

import quadra.core
import quadra.prim
import quadra.kit
import quadra.kit.input
import quadra.text
import quadra.ui

@[heap]
pub struct LogView {
pub mut:
	id         string
	store      LogStore
	mode       FilterMode
	query      string
	use_re     bool
	follow     bool = true
	timestamps bool = true
	focus      core.FocusHandle
	error      string // a bad regex
mut:
	matcher Matcher
	index   []int // matching lines, in hide mode with a query
	filter  core.View[input.TextInput]
	// view state, written by prepaint
	scroll prim.Point
	bounds prim.Rect
	cell   prim.Size
	base   f32
	gutter f32
	style  text.TextStyle
	widest f32
}

@[params]
pub struct LogOptions {
pub:
	id         string = 'log'
	timestamps bool   = true
}

pub fn new_log_view(mut cx core.Cx, opts LogOptions) core.View[LogView] {
	return cx.new_view(LogView{
		id:         opts.id
		timestamps: opts.timestamps
		focus:      cx.focus_handle()
	})
}

// append_logs adds lines without a closure (live sources call it every tick).
pub fn append_logs(mut cx core.Cx, v core.View[LogView], lines []LogEntry) {
	if lines.len == 0 {
		return
	}
	mut lv := cx.peek(v.model())
	lv.append(lines)
	cx.touch(v.model())
}

pub fn clear_logs(mut cx core.Cx, v core.View[LogView]) {
	mut lv := cx.peek(v.model())
	lv.store.clear()
	lv.index.clear()
	lv.scroll = prim.Point{}
	lv.widest = 0
	lv.follow = true
	cx.touch(v.model())
}

pub fn (mut lv LogView) append(lines []LogEntry) {
	lv.append_from(&lines)
}

// append_from walks the batch through a pointer: with the array itself in scope, every call
// in the loop would pin all of it again (specs/14, GC pins around calls).
fn (mut lv LogView) append_from(lines &[]LogEntry) {
	for k in 0 .. lines.len {
		e := unsafe { lines[k] }
		i := lv.store.len()
		lv.store.push(e.at_ms, e.text)
		if lv.indexed() && lv.matcher.matches(e.text) {
			lv.index << i
		}
	}
}

// set_filter changes the query (or the regex flag) and rescans the store once.
pub fn (mut lv LogView) set_filter(query string, use_re bool) {
	lv.query, lv.use_re, lv.error = query, use_re, ''
	lv.matcher = new_matcher(query, use_re) or {
		lv.error = err.msg()
		Matcher{}
	}
	lv.index.clear()
	if lv.indexed() {
		for i in 0 .. lv.store.len() {
			if lv.matcher.matches(lv.store.line(i)) {
				lv.index << i
			}
		}
	}
}

fn (lv &LogView) indexed() bool {
	return lv.mode == .hide && lv.matcher.active
}

// shown_count and shown_line walk the listed lines (all, or the matching ones).
pub fn (lv &LogView) shown_count() int {
	return if lv.indexed() { lv.index.len } else { lv.store.len() }
}

pub fn (lv &LogView) shown_line(k int) int {
	return if lv.indexed() { lv.index[k] } else { k }
}

@[markused]
pub fn (mut lv LogView) render(mut cx core.ViewCx[LogView]) core.Element {
	t := kit.theme(cx)
	if lv.filter.id == 0 {
		lv.filter = input.new_input(mut cx.Cx, id: '${lv.id}-filter', placeholder: 'Filter')
		input.on_change(mut cx.Cx, lv.filter, cx.listener(fn (mut lv LogView, s &string, mut cx core.ViewCx[LogView]) {
			lv.set_filter(*s, lv.use_re)
			cx.notify()
		}))
	}
	n, total := lv.shown_count(), lv.store.len()
	count := if lv.error != '' {
		'Bad pattern'
	} else if lv.matcher.active && lv.mode == .hide {
		'${n} of ${total}'
	} else {
		'${total} lines'
	}
	bar := ui.div().flex_row().items_center().gap(4).px(8).h(45).shrink_0().bg(t.p.canvas).border_b(1).border_color(t.p.line).child(ui.div().w(240).flex_col().child(cx.child_view(lv.filter))).child(kit.toggle_button('${lv.id}-regex',
		lv.use_re).label('.*').size(.sm).tooltip('Regular expression').on_toggle(cx.listener(fn (mut lv LogView, on &bool, mut cx core.ViewCx[LogView]) {
		lv.set_filter(lv.query, *on)
		cx.notify()
	}))).child(kit.toggle_button('${lv.id}-dim', lv.mode == .dim).label('Dim').size(.sm).tooltip('Dim other lines instead of hiding them').on_toggle(cx.listener(fn (mut lv LogView, on &bool, mut cx core.ViewCx[LogView]) {
		lv.mode = if *on { FilterMode.dim } else { FilterMode.hide }
		lv.set_filter(lv.query, lv.use_re)
		cx.notify()
	}))).child(ui.div().grow()).child(ui.text(count).size(t.ui_text(.sm)).color(t.p.text_3)).child(kit.toggle_button('${lv.id}-follow',
		lv.follow).label('Follow').size(.sm).tooltip('Keep the newest line in view').on_toggle(cx.listener(fn (mut lv LogView, on &bool, mut cx core.ViewCx[LogView]) {
		lv.follow = *on
		cx.notify()
	})))
	return ui.div().id(lv.id).test_id(lv.id).flex_col().size_full().track_focus(lv.focus).child(bar).child(ui.div().grow().on_scroll(cx.listener(fn (mut lv LogView, e &core.ScrollEvent, mut cx core.ViewCx[LogView]) {
		lv.wheel(e, mut cx)
	})).child(&LogSurface{
		lv: &lv
	}))
}

fn (mut lv LogView) max_y() f32 {
	return prim.max(0, f32(lv.shown_count()) * lv.cell.height - lv.bounds.height)
}

fn (mut lv LogView) wheel(e &core.ScrollEvent, mut cx core.ViewCx[LogView]) {
	sideways := e.mods.has(.shift)
	dx := if e.dx == 0 && sideways { e.dy } else { e.dx }
	dy := if sideways { f32(0) } else { e.dy }
	before := lv.scroll
	max_x := prim.max(0, lv.widest - lv.bounds.width + lv.cell.width * 4)
	lv.scroll = prim.Point{prim.clamp(lv.scroll.x - dx, 0, max_x), prim.clamp(lv.scroll.y - dy, 0,
		lv.max_y())}
	// Scrolling up leaves follow mode; reaching the bottom again resumes it.
	if dy > 0 {
		lv.follow = false
	} else if lv.scroll.y >= lv.max_y() - 1 {
		lv.follow = true
	}
	if lv.scroll != before {
		cx.stop_propagation() // at an edge the page around it scrolls instead
		cx.notify()
	}
}
