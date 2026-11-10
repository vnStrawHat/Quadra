// The gutter (specs/22): `[HH:MM:SS] 123` left of the grid, stamped when output arrives so hidden
// tabs keep accurate times. Grow-only per absolute line, as in the original.
module main

import time
import quadra.core
import quadra.kit
import quadra.kit.term
import quadra.prim
import quadra.ui

const gutter_pad = f32(8) // the labels sit half of it in from the left, half before the grid

// Gutter stamps absolute lines with the time they first held content.
// ponytail: an array shifted from the front, one line at a time past the scrollback cap; a ring
// if a huge scrollback makes the shift show up in profiles.
struct Gutter {
mut:
	times []int // seconds of day: times[j] belongs to the absolute line base + j
	base  int
	lines int // line feeds seen: the line numbers survive scrollback eviction
	past  int // the history length last seen
}

fn seconds_of_day() int {
	n := time.now()
	return n.hour * 3600 + n.minute * 60 + n.second
}

// absolute is the line count the numbers run to, never fewer than the rows the grid holds.
fn (g &Gutter) absolute(t &term.Terminal) int {
	rows := t.screen.scrollback.len() + t.screen.rows
	return if g.lines > rows { g.lines } else { rows }
}

fn content_row(t &term.Terminal) int {
	base := t.screen.scrollback.len()
	mut r := t.screen.rows - 1
	for r >= 0 {
		if t.row_at(base + r).cells.any(it.ch != 0 && it.ch != 32) {
			return r
		}
		r--
	}
	return 0
}

// stamp runs after each chunk of output. Empty rows below the content wait for content.
fn (mut g Gutter) stamp(t &term.Terminal, data []u8, now int) {
	g.lines += data.count(it == `\n`)
	history := t.screen.scrollback.len()
	if history == 0 && g.past > 0 { // a clear: new lines must not hit old stamps
		g.times.clear()
	}
	g.past = history
	rows := t.screen.rows
	absolute := g.absolute(t)
	oldest := absolute - history - rows
	if g.times.len == 0 {
		g.base = oldest
	}
	cursor := if t.screen.y > 0 { t.screen.y } else { 0 }
	content := content_row(t)
	row := if content > cursor { content } else { cursor }
	high := absolute - rows + row + 1
	for g.base + g.times.len < high {
		g.times << now
	}
	if oldest > g.base {
		drop := if oldest - g.base < g.times.len { oldest - g.base } else { g.times.len }
		g.times.delete_many(0, drop)
		g.base += drop
	}
}

fn gutter_digits(count int) int {
	return if count < 100 { 2 } else { count.str().len }
}

// label is one display row's clock and number.
fn (g &Gutter) label(absolute int, offset int, rows int, i int) (string, string) {
	idx := absolute - offset - rows + i
	num := if idx + 1 < 1 { 1 } else { idx + 1 }
	mut clock := '--:--:--'
	if idx >= 0 && g.times.len > 0 {
		t := if idx >= g.base {
			j := idx - g.base
			if j < g.times.len {
				g.times[j]
			} else {
				g.times.last()
			}
		} else {
			g.times[0]
		}
		clock = '${t / 3600 % 24:02}:${t / 60 % 60:02}:${t % 60:02}'
	}
	digits := gutter_digits(absolute)
	text := num.str()
	return '[${clock}]', if text.len < digits {
		' '.repeat(digits - text.len) + text
	} else {
		text
	}
}

fn half_light(c prim.Color) prim.Color {
	return prim.Color{c.r * 0.5, c.g * 0.5, c.b * 0.5, c.a}
}

// gutter_column is the labels and the column background; also fits the grid's left padding.
fn (mut x TermExt) gutter_column(mut t term.Terminal, th &kit.Theme) core.Element {
	base_left := x.one.cfg.layout.padding.left
	digits := gutter_digits(x.gutter.absolute(t))
	width := if t.cell.width > 0 { f32(11 + digits) * t.cell.width + gutter_pad } else { 0 }
	if t.b.padding.left != base_left + width {
		t.b.padding.left = base_left + width
	}
	c := x.one.cfg.colors
	bg := th.code.terminal.background
	fg := half_light(th.code.terminal.foreground)
	gutter_fg := prim.parse_hex(c.gutter_fg) or { fg }
	clock := prim.parse_hex(c.clock_fg) or { gutter_fg }
	numbers := prim.parse_hex(c.line_number_fg) or { gutter_fg }
	mut col := ui.div().absolute().top(0).left(0).bottom(0).w(width).test_id('${t.id}-gutter').bg(prim.parse_hex(c.gutter_bg) or {
		bg
	})
	family := if t.b.family != '' { t.b.family } else { th.mono_family }
	size := th.scaled((if t.b.size > 0 { t.b.size } else { term.font_size }) + t.zoom)
	absolute := x.gutter.absolute(t)
	for r in 0 .. t.screen.rows {
		left, right := x.gutter.label(absolute, t.offset, t.screen.rows, r)
		col.child(ui.div().absolute().left(gutter_pad / 2).top(t.b.padding.top +
			f32(r) * t.cell.height).h(t.cell.height).flex_row().items_center().gap(t.cell.width).child(ui.text(left).family(family).size(size).color(clock)).child(ui.text(right).family(family).size(size).color(numbers)))
	}
	return col
}

// layers is what the terminal draws over its grid: the gutter and the completion overlay.
fn (mut x TermExt) layers(mut t term.Terminal, th &kit.Theme, mut cx core.ViewCx[term.Terminal]) ?core.Element {
	show_gutter := x.one.cfg.layout.show_gutter
	if !show_gutter && !x.find.open {
		base_left := x.one.cfg.layout.padding.left
		if t.b.padding.left != base_left {
			t.b.padding.left = base_left
		}
		return x.overlay_element(t, th)
	}
	mut box := ui.div().absolute().top(0).left(0).size_full()
	if show_gutter {
		box.child(x.gutter_column(mut t, th))
	}
	if over := x.overlay_element(t, th) {
		box.child(over)
	}
	if x.find.open {
		box.child(x.find_bar(t, th, mut cx))
	}
	return box
}

fn ext_output(holder voidptr, mut t term.Terminal, data []u8) {
	mut x := unsafe { &TermExt(holder) }
	x.gutter.stamp(t, data, seconds_of_day())
}

fn (mut o OneTerm) toggle_gutter(mut cx core.ViewCx[OneTerm]) {
	o.cfg.layout.show_gutter = !o.cfg.layout.show_gutter
	o.apply_and_persist(mut cx.Cx)
}
