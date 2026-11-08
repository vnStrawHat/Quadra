// Workload status marks and the status bar (specs/15): every status has a color and a
// shape, so it reads in grayscale and for color-blind users.
module kit

import quadra.core
import quadra.prim
import quadra.ui

pub enum Status {
	running
	pending
	failed
	succeeded
	terminating
	unknown
}

pub fn (s Status) str() string {
	return match s {
		.running { 'Running' }
		.pending { 'Pending' }
		.failed { 'Failed' }
		.succeeded { 'Succeeded' }
		.terminating { 'Terminating' }
		.unknown { 'Unknown' }
	}
}

// status_mark is the shape for a status, about 10 px: dot, ring, triangle, check, dash,
// square.
pub fn status_mark(s Status) &Styled {
	return themed_with(s, fn (s Status, t &Theme) core.Element {
		p := t.p
		mark := match s {
			.running { core.Element(ui.div().w(8).h(8).rounded(4).bg(p.running)) }
			.pending { core.Element(ui.div().w(8).h(8).rounded(4).border(2).border_color(p.pending)) }
			.failed { core.Element(icon(icon_triangle_alert).size(12).color(p.failed)) }
			.succeeded { core.Element(icon(icon_check).size(12).color(p.succeeded)) }
			.terminating { core.Element(ui.div().w(8).h(2).rounded(1).bg(p.terminating)) }
			.unknown { core.Element(ui.div().w(7).h(7).rounded(1).bg(p.unknown)) }
		}

		return ui.div().w(12).h(12).center().shrink_0().child(mark)
	})
}

// status_label is a mark followed by its name (or a custom text), in the status color.
pub fn status_label(s Status, text string) &Styled {
	return themed_with(StatusText{s, text}, fn (a StatusText, t &Theme) core.Element {
		s, text := a.s, a.text
		name := if text == '' { s.str() } else { text }
		return ui.div().flex_row().items_center().gap(6).child(status_mark(s)).child(ui.text(name).color(status_color(t,
			s)))
	})
}

pub fn status_color(t &Theme, s Status) prim.Color {
	p := t.p
	return match s {
		.running { p.running }
		.pending { p.pending }
		.failed { p.failed }
		.succeeded { p.succeeded }
		.terminating { p.terminating }
		.unknown { p.unknown }
	}
}

// statusbar is the window's bottom strip (specs/15): 30 px on the chrome tone, 12 px
// items on the left and right.
pub fn statusbar(left []core.Element, right []core.Element) &Styled {
	return themed_with(Sides{left, right}, fn (a Sides, t &Theme) core.Element {
		left, right := a.left, a.right
		return ui.div().test_id('statusbar').flex_row().items_center().gap(12).px(8).h(t.statusbar_h).shrink_0().bg(t.p.status_bg).border_t(1).border_color(t.p.line_strong).text_size(t.font_size(.sm)).text_color(t.p.text).children(left).child(ui.div().grow()).children(right)
	})
}

// status_item is one status bar entry: an optional icon and a text.
pub fn status_item(glyph string, text string) &Styled {
	return themed_with(StatusItemArgs{glyph, text}, fn (a StatusItemArgs, t &Theme) core.Element {
		glyph, text := a.glyph, a.text
		mut d := ui.div().flex_row().items_center().gap(4)
		if glyph != '' {
			d.child(icon(glyph).size(14).color(t.p.text_2))
		}
		return d.child(ui.text(text))
	})
}

// status_button is an icon-only status bar toggle (specs/15): 22 px, a 14 px muted icon,
// the accent while its panel is open.
pub fn status_button(id string, glyph string, active bool) &Button {
	return icon_button(id, glyph).muted().selected(active)
}

// status_divider separates groups of status bar buttons: 1 x 14 px in the border color.
pub fn status_divider() &Styled {
	return themed(fn (t &Theme) core.Element {
		return ui.div().w(1).h(14).shrink_0().bg(t.p.line_strong)
	})
}

// status_of maps a status word (Running, Pending, ...) to a Status; others are unknown.
pub fn status_of(word string) Status {
	return match word {
		'Running' { .running }
		'Pending' { .pending }
		'Failed', 'Error', 'CrashLoopBackOff' { .failed }
		'Succeeded', 'Completed' { .succeeded }
		'Terminating' { .terminating }
		else { .unknown }
	}
}

struct StatusText {
	s    Status
	text string
}

struct Sides {
	left  []core.Element
	right []core.Element
}

struct StatusItemArgs {
	glyph string
	text  string
}
