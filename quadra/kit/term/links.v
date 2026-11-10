// Links: OSC 8 hyperlinks, and opening any link through the policy (specs/22).
module term

import quadra.core
import quadra.kit

// Link is a link on the grid: from (line, c0) to (end_line, c1), one row unless it wraps.
pub struct Link {
pub:
	line     int
	c0       int
	c1       int
	url      string
	end_line int
	display  string // the visible label of an OSC 8 link
	osc8     bool
}

// LinkTarget is one OSC 8 link: cells with the same explicit id and url form one link.
pub struct LinkTarget {
pub:
	id  string // '' when the program gave none
	url string
}

// CellExtra is what Cell.pad points at, interned so one id per cell covers them all.
pub struct CellExtra {
pub:
	link u16 // 1 + index into Screen.links
	ul   u32 // underline color, 0 = none
	over bool
}

@[inline]
pub fn (s &Screen) extra_of(pad u16) CellExtra {
	return if pad == 0 || int(pad) > s.extras.len { CellExtra{} } else { s.extras[pad - 1] }
}

// intern_extra returns the id for e, 0 when the table is full (the text still renders).
fn (mut s Screen) intern_extra(e CellExtra) u16 {
	if e == CellExtra{} {
		return 0
	}
	key := u64(e.link) << 40 | u64(e.ul) | if e.over { u64(1) << 32 } else { u64(0) }
	if id := s.extra_ids[key] {
		return id
	}
	if s.extras.len >= 0xfffe {
		return 0
	}
	s.extras << e
	id := u16(s.extras.len)
	s.extra_ids[key] = id
	return id
}

// set_link is OSC 8. Params hold `id=...` among `:`-separated pairs; an empty url ends the
// link. The same explicit id and url share an entry; without an id every link is its own.
fn (mut s Screen) set_link(params string, url string) {
	mut e := s.extra_of(s.pen.pad)
	mut link := 0
	if url != '' && url.len <= 2083 {
		mut id := ''
		for pair in params.split(':') {
			if pair.starts_with('id=') {
				id = pair[3..]
				break
			}
		}
		if id != '' {
			link = s.links.index(LinkTarget{id, url}) + 1
		}
		if link == 0 && s.links.len < 0xfffe {
			s.links << LinkTarget{id, url}
			link = s.links.len
		}
	}
	e = CellExtra{
		...e
		link: u16(link)
	}
	s.pen.pad = s.intern_extra(e)
}

fn (s &Screen) link_id(c Cell) ?u16 {
	id := s.extra_of(c.pad).link
	return if id != 0 && int(id) <= s.links.len { id } else { none }
}

// link_at returns the link under a cell.
fn (t &Terminal) link_at(p Pos) ?Link {
	cells := t.row_at(p.line).cells
	if p.col >= cells.len {
		return none
	}
	id := t.screen.extra_of(cells[p.col].pad).link
	if id == 0 || int(id) > t.screen.links.len {
		return t.plain_url_at(p)
	}
	mut a, mut b := p.col, p.col + 1
	for a > 0 && t.screen.extra_of(cells[a - 1].pad).link == id {
		a--
	}
	for b < cells.len && t.screen.extra_of(cells[b].pad).link == id {
		b++
	}
	mut label := []rune{cap: b - a}
	for x in a .. b {
		if cells[x].flags & f_spacer == 0 {
			label << visible_rune(cells[x])
		}
	}
	return Link{
		line:     p.line
		c0:       a
		c1:       b
		url:      t.screen.links[id - 1].url
		end_line: p.line
		display:  label.string()
		osc8:     true
	}
}

// open_link opens a clicked link after the target policy decided: allowed links open, a
// confirmation asks first, a denied link is logged.
fn (mut t Terminal) open_link(l Link, mut cx core.ViewCx[Terminal]) {
	display := if l.osc8 { l.display } else { '' }
	d := validate_target_with_display(l.url, display)
	match d.verdict {
		.allow {
			cx.open_external(l.url)
		}
		.confirm {
			t.pending_url = l.url
			kit.confirm(mut cx.Cx, 'Open link?', url_confirmation_text(l.url, display), 'Open', cx.listener(fn (mut t Terminal, ok &bool, mut cx core.ViewCx[Terminal]) {
				if *ok {
					cx.open_external(t.pending_url)
				}
				t.pending_url = ''
			}))
		}
		.deny {
			eprintln('terminal: URL denied: ${d.why} — ${l.url}')
		}
	}
}
