// Engine conformance (specs/22): replays terminal recordings and compares every cell with the
// frozen expectation. The corpus is fetched by tools/setup.sh; without it this test is skipped.
module main

import json
import os
import quadra.kit.term

const corpus_root = os.join_path(@DIR, '..', '.cache', 'downloads', 'oneterm', 'crates', 'tools',
	'corpus')
const corpora = ['alacritty-ref', 'oneterm']

struct Size {
	columns      int
	screen_lines int
}

struct History {
	history_size int
}

const named = {
	'Black':         0
	'Red':           1
	'Green':         2
	'Yellow':        3
	'Blue':          4
	'Magenta':       5
	'Cyan':          6
	'White':         7
	'BrightBlack':   8
	'BrightRed':     9
	'BrightGreen':   10
	'BrightYellow':  11
	'BrightBlue':    12
	'BrightMagenta': 13
	'BrightCyan':    14
	'BrightWhite':   15
}

// color_of turns an expectation's color into the engine's encoding.
fn color_of(s string) u32 {
	if s[0] == `#` {
		return term.true_color | u32(s[1..].parse_uint(16, 32) or { 0 })
	}
	if s[0] == `i` {
		return term.palette(s[1..].int())
	}
	return if i := named[s[1..]] { term.palette(i) } else { 0 } // nForeground, nBackground
}

// percent_encode matches the expectation files' encoding of links and titles.
fn percent_encode(text string) string {
	mut out := []u8{cap: text.len}
	for b in text.bytes() {
		if b.is_alnum() || b in [`-`, `.`, `_`, `/`, `:`] {
			out << b
		} else {
			out << '%${b:02X}'.bytes()
		}
	}
	return out.bytestr()
}

fn attrs_of(c term.Cell, wrap bool) string {
	underlined := c.flags & term.f_underline != 0
	styled := c.flags & (term.f_double | term.f_curly | term.f_dotted | term.f_dashed) != 0
	flags := [c.flags & term.f_inverse != 0, c.flags & term.f_bold != 0, c.flags & term.f_italic != 0,
		underlined && !styled, wrap, c.flags & term.f_wide != 0, c.flags & term.f_spacer != 0,
		c.flags & term.f_faint != 0, c.flags & term.f_hidden != 0, c.flags & term.f_strike != 0,
		underlined && c.flags & term.f_double != 0, underlined && c.flags & term.f_curly != 0,
		underlined && c.flags & term.f_dotted != 0, underlined && c.flags & term.f_dashed != 0]
	names := ['INVERSE', 'BOLD', 'ITALIC', 'UNDERLINE', 'WRAPLINE', 'WIDE_CHAR', 'WIDE_CHAR_SPACER',
		'DIM', 'HIDDEN', 'STRIKEOUT', 'DOUBLE_UNDERLINE', 'UNDERCURL', 'DOTTED_UNDERLINE',
		'DASHED_UNDERLINE']
	mut out := []string{}
	for i, on in flags {
		if on {
			out << names[i]
		}
	}
	return if out.len == 0 { '-' } else { out.join('.') }
}

// row_newest returns row `index` counted from the bottom of the screen into the history.
fn row_newest(s &term.Screen, index int) ?term.Row {
	if index < s.rows {
		return s.lines[s.rows - 1 - index]
	}
	h := index - s.rows
	n := s.scrollback.len()
	if h >= n {
		return none
	}
	return term.unpack(s.scrollback.at(n - 1 - h), s.cols)
}

// rgb6 spells a cell color as #rrggbb.
fn rgb6(v u32) string {
	h := (v & 0xffffff).hex()
	return '#' + '000000'[h.len..] + h
}

// marks_at is the combining marks on a cell, as the expectation spells them.
fn marks_at(row term.Row, col int) string {
	mut out := ''
	for m in row.marks {
		if m.col == col {
			for r in m.s.runes() {
				out += '+${u32(r):04x}'
			}
		}
	}
	return out
}

// link_key is a cell's link as the file spells it, minus the number of an implicit link:
// the recordings give each occurrence its own, the original (its deviation D17) and Quadra
// one per URI.
fn link_key(e term.CellExtra, s &term.Screen) string {
	if e.link == 0 {
		return '-'
	}
	t := s.links[e.link - 1]
	id := if t.id == '' { '#' } else { percent_encode(t.id) }
	return '${id}~${percent_encode(t.url)}'
}

// first_difference compares the grid with grid.expect and returns the first mismatch, or ''.
fn first_difference(s &term.Screen, expect string) string {
	blank := term.Cell{}
	for line in expect.split_into_lines() {
		if !line.starts_with('row ') {
			continue
		}
		parts := line.split_nth(' ', 4)
		index := parts[1].int()
		want_wrap := parts[2] == 'wrap=1'
		row := row_newest(s, index) or { term.Row{} }
		got_wrap := row.wrapped
		if want_wrap != got_wrap {
			return 'row ${index}: wrap ${got_wrap}, want ${want_wrap}'
		}
		mut col := 0
		for run in parts[3].split('|') {
			star := run.index('*') or { continue }
			count := run[..star].int()
			f := run[star + 1..].split(';')
			want_ch := f[0].all_before('+').parse_uint(16, 32) or { 0 }
			want_fg, want_bg := color_of(f[2]), color_of(f[3])
			want_marks := if f[0].contains('+') { '+' + f[0].all_after('+') } else { '' }
			want_ul := f[4]
			want_link := if f[5].starts_with('#') { '#~' + f[5].all_after('~') } else { f[5] }
			for _ in 0 .. count {
				c := if col < row.cells.len { row.cells[col] } else { blank }
				last := col == s.cols - 1
				ch := if c.ch == 0 { u64(32) } else { u64(c.ch) }
				got_attrs := attrs_of(c, want_wrap && last)
				e := s.extra_of(c.pad)
				mut ul := '-'
				if e.ul != 0 {
					ul = if e.ul & term.true_color != 0 {
						rgb6(e.ul)
					} else {
						'i${e.ul - 1}'
					}
				}
				got_link := link_key(e, s)
				got_marks := if col < row.cells.len { marks_at(row, col) } else { '' }
				if ul != want_ul || got_link != want_link || got_marks != want_marks {
					return 'row ${index} col ${col}: underline ${ul} link ${got_link} marks ${got_marks}, want ${run[
						star + 1..]}'
				}
				if ch != want_ch || c.fg != want_fg || c.bg != want_bg || got_attrs != f[1] {
					return 'row ${index} col ${col}: U+${ch.hex()} ${got_attrs} fg ${c.fg.hex()} bg ${c.bg.hex()}, want ${run[
						star + 1..]}'
				}
				col++
			}
		}
	}
	return ''
}

// mode_names mirrors the state.expect `modes` line.
fn mode_names(s &term.Screen) string {
	flags := [s.alt_scroll_on(), s.alt_active, s.app_cursor, s.app_keypad, s.bracketed, s.focus_reports,
		s.insert, s.autowrap, s.mouse == .drag, s.mouse == .any, s.mouse == .click, s.origin,
		s.mouse_enc == .sgr, s.cursor_on, s.urgency, s.mouse_enc == .utf8]
	names := ['ALTERNATE_SCROLL', 'ALT_SCREEN', 'APP_CURSOR', 'APP_KEYPAD', 'BRACKETED_PASTE',
		'FOCUS_IN_OUT', 'INSERT', 'LINE_WRAP', 'MOUSE_DRAG', 'MOUSE_MOTION', 'MOUSE_REPORT_CLICK',
		'ORIGIN', 'SGR_MOUSE', 'SHOW_CURSOR', 'URGENCY_HINTS', 'UTF8_MOUSE']
	mut out := []string{}
	for i, on in flags {
		if on {
			out << names[i]
		}
	}
	return out.join(' ')
}

// state_difference compares the state.expect lines this engine stores.
fn state_difference(s &term.Screen, state string) string {
	for line in state.split_into_lines() {
		if line == '' || line.starts_with('#') || line.starts_with('version') {
			continue
		}
		key := line.all_before(' ')
		want := line.all_after_first(' ')
		mut got := ''
		if key == 'cursor' {
			shape := match s.shape {
				.block { 'Block' }
				.underline { 'Underline' }
				.bar { 'Beam' }
			}

			shown := if s.cursor_on { shape } else { 'Hidden' }
			got = 'row=${s.y} col=${s.x} shape=${shown}'
		} else if key == 'pending_wrap' {
			got = if s.wrap_pending { '1' } else { '0' }
		} else if key == 'modes' {
			got = mode_names(s)
		} else if key == 'title_events' {
			got = s.title_count.str()
		} else if key == 'scroll_region' {
			got = 'top=${s.top} bottom=${s.bottom}'
		} else if key == 'tab_stops' {
			mut stops := []string{}
			for x in 1 .. s.cols - 1 {
				if s.has_stop(x) {
					stops << x.str()
				}
			}
			if s.cols > 1 {
				stops << '${s.cols - 1}'
			}
			got = stops.join(',')
		} else if key.starts_with('palette.') {
			got = rgb6(s.over[key.all_after('palette.').int()])
		} else {
			continue
		}
		if got != want {
			return '${key}: ${got}, want ${want}'
		}
	}
	return ''
}

// replay feeds the recording in chunks of `chunk` bytes (0: whole), so sequences split
// across reads take the parser's byte loop instead of its fast paths.
fn replay(corpus string, name string, chunk int) !string {
	dir := os.join_path(corpus, name)
	size := json.decode(Size, os.read_file(os.join_path(dir, 'size.json'))!)!
	cfg := json.decode(History, os.read_file(os.join_path(dir, 'config.json'))!)!
	mut s := term.new_screen(size.columns, size.screen_lines)
	s.scrollback.max = cfg.history_size
	mut p := term.Parser{}
	data := os.read_bytes(os.join_path(dir, 'recording'))!
	step := if chunk > 0 { chunk } else { data.len }
	for i := 0; i < data.len; i += step {
		p.feed(mut s, data[i..int_min(i + step, data.len)])
	}
	diff := first_difference(s, os.read_file(os.join_path(dir, 'grid.expect'))!)
	if diff != '' {
		return diff
	}
	return state_difference(s, os.read_file(os.join_path(dir, 'state.expect'))!)
}

fn test_recordings_match() {
	if !os.exists(corpus_root) {
		eprintln('skipped: no corpus (tools/setup.sh fetches it)')
		return
	}
	mut replayed := 0
	for set in corpora {
		dir := os.join_path(corpus_root, set)
		for name in os.ls(dir)! {
			for chunk in [0, 1, 13] {
				diff := replay(dir, name, chunk)!
				assert diff == '', '${set}/${name} (chunks of ${chunk}): ${diff}'
			}
			replayed++
		}
	}
	assert replayed >= 46
}
