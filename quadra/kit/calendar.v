// Month calendar: the grid the date picker shows in its popup, and Calendar, the same
// grid on its own with its shown month kept in a view.
module kit

import quadra.core
import quadra.layout
import quadra.ui

// CalendarTarget is what a month grid's day cells and arrows act on: an object, so the
// cells need no closures (specs/14).
interface CalendarTarget {
	pick(d Date, mut cx core.Cx)
	nav(y int, m int, mut cx core.Cx)
}

struct DayClick {
	target CalendarTarget
	date   Date
}

fn day_click(d DayClick, _ &core.ClickEvent, mut cx core.Cx) {
	d.target.pick(d.date, mut cx)
}

struct NavClick {
	target CalendarTarget
	y      int
	m      int
}

fn nav_click(n NavClick, _ &core.ClickEvent, mut cx core.Cx) {
	n.target.nav(n.y, n.m, mut cx)
}

struct MonthGrid {
	prefix string
	year   int
	month  int
	value  Date
	words  Strings
	target CalendarTarget
}

// month_grid is the calendar of (year, month) with value highlighted. A day click calls
// target.pick, the month arrows target.nav. Test ids start with prefix.
fn month_grid(prefix string, year int, month int, value Date, words Strings, target CalendarTarget) core.Element {
	return themed_with(MonthGrid{prefix, year, month, value, words, target}, build_month_grid)
}

fn cal_arrow(g MonthGrid, y int, m int, glyph string, id string) core.Element {
	return icon_button(g.prefix + id, glyph).size(.sm).on_click(core.handler_with(NavClick{g.target, y, m},
		nav_click))
}

fn build_month_grid(g MonthGrid, t &Theme) core.Element {
	p := t.p
	year, month, value, prefix := g.year, g.month, g.value, g.prefix
	prev_y, prev_m := if month == 1 { year - 1, 12 } else { year, month - 1 }
	next_y, next_m := if month == 12 { year + 1, 1 } else { year, month + 1 }
	// Derived (specs/15): ghost day cells 28 tall, the chosen day tinted with an accent
	// label, weekday names muted at 12 px.
	mut grid := ui.div().grid().cols(layout.repeat(7, layout.track_px(32))).gap(2)
	for n in g.words.weekdays {
		grid.child(ui.div().h(22).center().child(ui.text(n).size(t.font_size(.sm)).color(p.text_2)))
	}
	for _ in 0 .. first_weekday(year, month) {
		grid.child(ui.div())
	}
	for d in 1 .. days_in(year, month) + 1 {
		on := value.year == year && value.month == month && value.day == d
		date := Date{year, month, d}
		mut cell := ui.div().test_id('${prefix}day-${d}').h(28).center().rounded(t.radius_md).cursor(.pointer).text_size(t.font_size(.md)).role(.button).label(date.str()).selected(on).on_click(core.handler_with(DayClick{g.target, date},
			day_click))
		if on {
			cell.bg(p.accent_tint).text_color(p.accent_text)
		} else {
			cell.hover(ui.Patch{
				background: p.panel_hover
			})
		}
		grid.child(cell.child(ui.text('${d}')))
	}
	head := ui.div().flex_row().items_center().child(cal_arrow(g, prev_y, prev_m,
		icon_chevron_left, 'cal-prev')).child(ui.div().grow().center().child(ui.text(g.words.month_year(month, year)))).child(cal_arrow(g,
		next_y, next_m, icon_chevron_right, 'cal-next'))
	return ui.div().flex_col().gap(8).p(8).bg(p.overlay_bg).rounded(t.radius_xl).border(1).border_color(p.line).shadow_level(1).child(head).child(grid)
}

// Calendar is a month calendar in the page. Its test ids are `<id>-day-N`, `<id>-cal-prev`
// and `<id>-cal-next`.
pub struct Calendar {
pub:
	id string
pub mut:
	year  int
	month int
	value Date
mut:
	change ?core.AnyListener
}

pub fn new_calendar(mut cx core.Cx, id string, value Date) core.View[Calendar] {
	return cx.new_view(Calendar{
		id:    id
		year:  value.year
		month: value.month
		value: value
	})
}

// on_date calls l when a day is picked (the calendar renders again to take it).
pub fn on_date(mut cx core.Cx, c core.View[Calendar], l core.Listener[Date]) {
	mut cal := cx.peek(c.model())
	cal.change = l.any
	cx.touch(c.model())
}

// InPage is a page calendar as its grid's target: picking or paging updates the view.
struct InPage {
	me     core.Model[Calendar]
	change ?core.AnyListener
}

fn (p InPage) pick(d Date, mut cx core.Cx) {
	mut c := cx.peek(p.me)
	c.value = d
	cx.touch(p.me)
	fire(p.change, d, mut cx)
}

fn (p InPage) nav(y int, m int, mut cx core.Cx) {
	mut c := cx.peek(p.me)
	c.year, c.month = y, m
	cx.touch(p.me)
}

@[markused]
pub fn (mut c Calendar) render(mut cx core.ViewCx[Calendar]) core.Element {
	return month_grid('${c.id}-', c.year, c.month, c.value, strings_of(cx.app), InPage{cx.handle().model(), c.change})
}
