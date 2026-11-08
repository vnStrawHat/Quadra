// Standalone calendar, time input and range slider (kit/calendar.v, time_input.v,
// range_slider.v).
module main

import quadra.core
import quadra.kit
import quadra.testing
import quadra.ui

struct Picks {
mut:
	cal    core.View[kit.Calendar]
	date   kit.Date
	time   kit.TimeOfDay = kit.TimeOfDay{9, 30}
	range  kit.Range     = kit.Range{20, 60}
	events int
}

@[markused]
pub fn (mut p Picks) render(mut cx core.ViewCx[Picks]) core.Element {
	if p.cal.id == 0 {
		p.cal = kit.new_calendar(mut cx.Cx, 'cal', kit.Date{2026, 11, 7})
		kit.on_date(mut cx.Cx, p.cal, cx.listener(fn (mut p Picks, d &kit.Date, mut _ core.ViewCx[Picks]) {
			p.date = *d
		}))
	}
	return ui.div().flex_col().gap(8).p(8).child(cx.child_view(p.cal)).child(kit.time_input('t',
		p.time).minute_step(15).on_change(cx.listener(fn (mut p Picks, t &kit.TimeOfDay, mut _ core.ViewCx[Picks]) {
		p.time = *t
	}))).child(kit.range_slider('r', p.range.lo, p.range.hi).width(216).on_change(cx.listener(fn (mut p Picks, r &kit.Range, mut _ core.ViewCx[Picks]) {
		p.range = *r
		p.events++
	})))
}

fn picks(h &testing.Harness, v core.View[Picks]) &Picks {
	cx := core.Cx{
		app: h.app
	}
	return cx.peek(v.model())
}

fn setup_picks() (&testing.Harness, core.View[Picks]) {
	mut h := testing.new_harness(width: 400, height: 600) or { panic(err) }
	kit.install(mut h.app) or { panic(err) }
	v := h.open(Picks{}, width: 400, height: 600) or { panic(err) }
	return h, v
}

fn test_calendar_keeps_its_month_and_picks_a_day() {
	mut h, v := setup_picks()
	h.click('cal-cal-next')!
	h.click('cal-cal-next')! // January 2027: the shown month survives re-renders
	h.click('cal-day-3')!
	assert picks(h, v).date == kit.Date{2027, 1, 3}
	cell := h.find_role(.button, '03/01/2027') or { panic('no selected day') }
	assert cell.selected
}

fn test_time_input_steps_wraps_and_types() {
	mut h, v := setup_picks()
	h.click('t-h')!
	h.press('down')!
	assert picks(h, v).time == kit.TimeOfDay{8, 30}
	h.press('2')!
	h.press('3')!
	assert picks(h, v).time == kit.TimeOfDay{23, 30}
	h.press('7')! // 37 is no hour: a new value
	assert picks(h, v).time.hour == 7
	h.click('t-m')!
	h.press('up')!
	h.press('up')!
	assert picks(h, v).time == kit.TimeOfDay{7, 0} // 30 + 2 * 15 wraps
	h.press('down')!
	assert picks(h, v).time.minute == 45
}

fn test_range_slider_moves_the_nearer_thumb_and_keeps_order() {
	mut h, v := setup_picks()
	r := h.find('r') or { panic('no slider') }
	// The track runs from x + 8 to x + 208 for 0..100: 2 px per unit.
	h.click_at(r.x + 8 + 30, r.y + 10) // 15: nearer the low thumb
	assert picks(h, v).range == kit.Range{15, 60}
	h.click_at(r.x + 8 + 180, r.y + 10) // 90
	assert picks(h, v).range == kit.Range{15, 90}
	h.click('r-lo')!
	h.press('end')! // stops at the high thumb
	assert picks(h, v).range == kit.Range{90, 90}
	h.press('home')!
	assert picks(h, v).range == kit.Range{0, 90}
	h.click('r-hi')!
	h.press('left')!
	assert picks(h, v).range == kit.Range{0, 89}
}
