// Date picker: a field showing the date that opens a month calendar.
module kit

import time
import quadra.core
import quadra.prim
import quadra.layout
import quadra.ui

pub struct Date {
pub:
	year  int
	month int // 1..12
	day   int
}

pub fn today() Date {
	t := time.now()
	return Date{t.year, t.month, t.day}
}

pub fn (d Date) str() string {
	return '${d.day:02}/${d.month:02}/${d.year}' // dd/mm/yyyy (vi-VN); i18n formats in quadra.app
}

fn days_in(year int, month int) int {
	return time.days_in_month(month, year) or { 30 }
}

// weekday of the first day of the month, Monday = 0.
fn first_weekday(year int, month int) int {
	t := time.new(year: year, month: month, day: 1)
	return (t.day_of_week() + 6) % 7
}

@[heap]
pub struct DatePicker {
	Lazy
pub mut:
	id     string
	value  Date
	change ?core.AnyListener
mut:
	div &ui.Div = unsafe { nil }
}

pub fn date_picker(id string, value Date) &DatePicker {
	return &DatePicker{
		id:    id
		value: value
	}
}

pub fn (self_ &DatePicker) on_change(l core.Listener[Date]) &DatePicker {
	mut d := unsafe { self_ }
	d.change = l.any
	return d
}

// InPopup is the date picker's popup as its grid's target: picking a day or another month
// closes it (another month opens it again).
struct InPopup {
	value  Date
	change ?core.AnyListener
	anchor prim.Rect
	words  Strings
}

fn (p InPopup) pick(d Date, mut cx core.Cx) {
	close_popups(mut cx)
	fire(p.change, d, mut cx)
}

fn (p InPopup) nav(y int, m int, mut cx core.Cx) {
	close_popups(mut cx)
	open_calendar(mut cx, p.anchor, y, m, p.value, p.change)
}

struct PopupMonth {
	year   int
	month  int
	target InPopup
}

fn popup_month(p PopupMonth, _ &Theme) core.Element {
	return month_grid('', p.year, p.month, p.target.value, p.target.words, p.target)
}

fn open_calendar(mut cx core.Cx, anchor prim.Rect, year int, month int, value Date, change ?core.AnyListener) {
	target := InPopup{value, change, anchor, strings_of(cx.app)}
	open_popup_with(mut cx, anchor, .bottom_start, PopupMonth{year, month, target}, popup_month)
}

fn open_picker(p InPopup, _ &core.ClickEvent, mut cx core.Cx) {
	open_calendar(mut cx, cx.target_bounds(), p.value.year, p.value.month, p.value, p.change)
}

pub fn (mut dp DatePicker) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut dp, mut cx)
	return dp.layout_inner(el, mut cx)
}

pub fn (mut dp DatePicker) build_tree(mut cx core.LayoutCx) core.Element {
	t := theme_of(cx.app)
	p := t.p
	value := dp.value
	change := dp.change
	mut d := ui.div().id(dp.id).test_id(dp.id).flex_row().items_center().gap(6).w(180).h(t.height(.md)).px(8).rounded(t.radius_md).border(1).border_color(p.line).cursor(.pointer).role(.button).label(value.str()).focus_ring(ui.Patch{
		border_color: p.border_focused
	})

	d.hover(ui.Patch{
		background:   p.panel_hover
		border_color: p.line_strong
	}).on_click(core.handler_with(InPopup{
		value:  value
		change: change
	}, open_picker))
	d.child(icon(icon_calendar).size(14).color(p.text_2)).child(ui.text(value.str()))
	dp.div = d
	return d
}

pub fn (mut dp DatePicker) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	dp.div.focus_id = stable_focus(mut cx, dp.id)
	dp.Lazy.prepaint(bounds, mut cx)
}
