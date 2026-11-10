// The semantic classes of the visible rows (specs/22, Highlight): one class per column, scanned
// per logical line, cached by the generations of its rows. Painting reads `Terminal.classes`.
module main

import highlight
import quadra.kit.term

const class_url = u8(highlight.Class.url)

// LineMark is what the OSC 133 zones on a logical line say; head is the first marked cell's.
struct LineMark {
	head   term.Zone
	closed bool // the shell closed its prompt with B on this line
}

// line_role is the original's rule: a prompt only when the region transitioned into it.
fn line_role(mark LineMark, prev ?LineMark, input_at int) (highlight.RowRole, int) {
	match mark.head {
		.prompt {
			if p := prev {
				if p.head != .prompt || p.closed {
					return highlight.RowRole.prompt, input_at
				}
			}
		}
		.output {
			return highlight.RowRole.output, -1
		}
		else {}
	}

	return highlight.RowRole.unmarked, -1
}

fn (mut x TermExt) reset_cache(rows int) {
	x.cache = [][]u8{len: rows}
	x.keys = []u64{len: rows, init: ~u64(0)}
	x.wraps = []bool{len: rows}
	x.visible = []term.Row{len: rows}
	x.roles.reset(rows)
}

fn (mut x TermExt) fill_classes(mut t term.Terminal, top int) {
	rows := t.screen.rows
	if x.cache.len != rows || t.classes.len != rows {
		t.classes = [][]u8{len: rows}
		x.reset_cache(rows)
	}
	if !x.sem.enabled {
		if !x.cleared {
			for i in 0 .. rows {
				t.classes[i] = []u8{}
			}
			x.cleared = true
		}
		return
	}
	x.cleared = false
	for i in 0 .. rows {
		x.visible[i] = t.row_at(top + i)
		x.wraps[i] = x.visible[i].wrapped
	}
	mut prev := ?LineMark(none)
	mut r := 0
	for r < rows {
		mut end := r
		for x.visible[end].wrapped && end + 1 < rows {
			end++
		}
		prev = x.scan_run(mut t, top, r, end, prev)
		r = end + 1
	}
	x.apply_exit_tint(mut t, rows)
}

// scan_run classifies rows start..end as one logical line, rescanning only when a row changed.
fn (mut x TermExt) scan_run(mut t term.Terminal, top int, start int, end int, prev ?LineMark) LineMark {
	mut head := term.Zone.none
	mut input_at := -1
	mut char_i := 0
	mut key := u64(0xcbf29ce484222325)
	for r in start .. end + 1 {
		row := x.visible[r]
		key = (key ^ u64(row.gen)) * 0x100000001b3
		for c in row.cells {
			if c.flags & term.f_spacer != 0 {
				continue
			}
			z := c.zone()
			if head == .none && z != .none {
				head = z
			}
			if input_at < 0 && z == .input {
				input_at = char_i
			}
			char_i++
		}
	}
	key = (key ^ u64(top + start)) * 0x100000001b3
	mark := LineMark{head, head == .prompt && input_at >= 0}
	role, at := line_role(mark, prev, input_at)
	key = (key ^ (u64(role) << 32 ^ u64(u32(at)))) * 0x100000001b3
	if key != x.keys[start] {
		x.keys[start] = key
		x.rescan(t, start, end, role, at)
	}
	for r in start .. end + 1 {
		t.classes[r] = x.cache[r]
		x.roles.role[r] = role
	}
	return mark
}

fn (mut x TermExt) rescan(t &term.Terminal, start int, end int, role highlight.RowRole, at int) {
	x.text.clear() // reused: a rescan per changed line must not allocate
	x.owners.clear()
	for r in start .. end + 1 {
		cells := x.visible[r].cells
		mut last := cells.len
		for r == end && last > 0 && (cells[last - 1].ch == 0 || cells[last - 1].ch == 32) {
			last--
		}
		for col in 0 .. last {
			if cells[col].flags & term.f_spacer == 0 {
				x.text << if cells[col].ch == 0 { ` ` } else { rune(cells[col].ch) }
				x.owners << (r << 16) | col
			}
		}
		if x.cache[r].len != cells.len {
			x.cache[r] = []u8{len: cells.len}
		} else {
			unsafe { vmemset(x.cache[r].data, 0, cells.len) }
		}
	}
	x.sem.scan_runes_into(x.text, role, at, mut x.scratch, mut x.out)
	for i, class in x.out {
		if i >= x.owners.len {
			break
		}
		r, col := x.owners[i] >> 16, x.owners[i] & 0xffff
		x.cache[r][col] = class
		if x.visible[r].cells[col].flags & term.f_wide != 0 && col + 1 < x.cache[r].len {
			x.cache[r][col + 1] = class
		}
	}
	for l in t.link_cols(x.visible[start..end + 1]) {
		for col in l.c0 .. l.c1 {
			x.cache[start + l.line][col] = class_url
		}
	}
}

// apply_exit_tint colors the sign of the last completed prompt, only at the bottom of the
// scrollback: scrolled up, the newest prompt on screen is somebody else's.
fn (mut x TermExt) apply_exit_tint(mut t term.Terminal, rows int) {
	x.sem.has_exit_code = t.offset == 0 && t.screen.last_exit != none
	if !x.sem.has_exit_code {
		return
	}
	code := t.screen.last_exit or { return }
	run := x.roles.last_completed_prompt(x.wraps) or { return }
	for r in run.start .. run.end {
		if r < rows && t.classes[r].len > 0 {
			mut c := t.classes[r].clone()
			highlight.tint_prompt_sign(mut c, code)
			t.classes[r] = c
		}
	}
}
