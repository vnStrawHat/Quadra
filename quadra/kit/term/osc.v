// OSC dispatch (specs/17). Everything an app reads comes out as a TerminalInfo.
module term

import encoding.base64

pub enum InfoKind {
	changed // title, cwd, search results: re-read the screen
	notification
	progress
	program_status
	shell_mark
	pointer
	icon_name
}

pub struct Progress {
pub:
	state   ProgressState
	percent int
}

pub enum ProgressState {
	remove
	set
	error
	indeterminate
	paused
}

// TerminalInfo is what a terminal tells its app (specs/17, Sequences the app reads).
pub struct TerminalInfo {
pub:
	kind     InfoKind
	text     string // notification body, pointer name, icon name
	warn     bool   // notification: a warning from the terminal itself
	progress Progress
	mark     u8   // shell_mark: which of mark_a..mark_d
	exit     ?int // shell_mark: the D exit code
}

pub struct ColorQuery {
pub:
	key int
	bel bool // the question ended with BEL
}

fn (mut s Screen) emit(ev TerminalInfo) {
	if s.events.len < 1024 {
		s.events << ev
	}
}

// osc_dispatch handles one complete OSC string. A trailing ST or BEL is already gone.
fn (mut p Parser) osc_dispatch(mut s Screen) {
	// Short payloads split fully, as the original's dispatch re-splits its last parameter; a
	// large one stops at 16 so it cannot make millions of strings.
	parts := p.osc.bytestr().split_nth(';', if p.osc_big { max_osc_params } else { 0 })
	match parts[0] {
		'0', '2' {
			if parts.len >= 2 {
				s.title = clean_text(parts[1..].join(';').trim_space(), max_title, true)
				s.title_count++
			}
		}
		'1' {
			if parts.len >= 2 {
				s.icon_name = clean_text(parts[1..].join(';').trim_space(), max_title, true)
				s.emit(TerminalInfo{ kind: .icon_name, text: s.icon_name })
			}
		}
		'4' {
			p.osc_palette(mut s, parts)
		}
		'7' {
			if parts.len >= 2 && !p.osc_cut {
				cwd := clean_text(osc_path(parts[1..].join(';')), max_cwd, false)
				if cwd != '' {
					s.cwd = cwd
				}
			}
		}
		'8' {
			if parts.len >= 3 {
				s.set_link(parts[1], parts[2..].join(';'))
			}
		}
		'9' {
			s.osc_nine(parts, p.osc_cut)
		}
		'10', '11', '12' {
			p.osc_dynamic(mut s, parts)
		}
		'17', '19' {
			p.osc_selection(mut s, parts)
		}
		'22' {
			if parts.len >= 2 && parts[1] != '' {
				s.emit(TerminalInfo{ kind: .pointer, text: parts[1] })
			}
		}
		'50' {
			s.osc_cursor_shape(parts)
		}
		'52' {
			s.osc_clipboard(parts)
		}
		'104' {
			s.osc_reset_palette(parts)
		}
		'110', '111', '112' {
			s.reset_color(key_fg + parts[0].int() - 110)
		}
		'133' {
			s.osc_shell(parts)
		}
		'7501' {
			p.osc_program_status(mut s, parts)
		}
		else {}
	}
}

// osc_nine is OSC 9: ConEmu progress under sub-code 4, a notification otherwise.
fn (mut s Screen) osc_nine(parts []string, cut bool) {
	if parts.len < 2 || (parts.len == 2 && parts[1] == '') {
		return
	}
	if parts[1] != '4' {
		s.emit(TerminalInfo{ kind: .notification, text: parts[1..].join(';') })
		return
	}
	if cut { // a cut number is a wrong number
		return
	}
	percent0 := clamp((if parts.len > 3 { parts[3].int() } else { 0 }), 0, 100)
	state_n := if parts.len > 2 { parts[2].int() } else { 0 }
	state := match state_n {
		0 { ProgressState.remove }
		1 { ProgressState.set }
		2 { ProgressState.error }
		3 { ProgressState.indeterminate }
		4 { ProgressState.paused }
		else { return }
	}

	percent := if state == .remove || state == .indeterminate { 0 } else { percent0 }
	s.progress = Progress{state, percent}
	s.emit(TerminalInfo{ kind: .progress, progress: s.progress })
}

// osc_cursor_shape is OSC 50 CursorShape=0|1|2, the only OSC 50 the terminal understands.
fn (mut s Screen) osc_cursor_shape(parts []string) {
	if parts.len < 2 || !parts[1].starts_with('CursorShape=') {
		return
	}
	s.shape = match parts[1].all_after('CursorShape=') {
		'1' { CursorShape.bar }
		'2' { CursorShape.underline }
		else { CursorShape.block }
	}
}

// osc_clipboard is OSC 52: selection c, p or s, base64 data or ?.
fn (mut s Screen) osc_clipboard(parts []string) {
	if parts.len < 3 || (parts[1] != '' && parts[1][0] !in [`c`, `p`, `s`]) {
		return
	}
	data := parts[2]
	if data == '?' {
		s.clipboard_query = s.allow_osc52_read
	} else if s.allow_osc52 && data.len <= max_osc52 / 3 * 4 + 4 {
		s.clipboard = base64.decode_str(data)
	}
}

// osc_path turns an OSC 7 argument into a local path: `file://host/path` URLs are decoded
// (`/C:/x` becomes `C:/x`), a raw path (cmd and PowerShell send those) is kept as it is.
pub fn osc_path(arg string) string {
	if !arg.starts_with('file://') {
		return arg
	}
	rest := arg[7..]
	path := percent_decode(if i := rest.index('/') { rest[i..] } else { '' })
	return if path.len > 2 && path[2] == `:` { path[1..] } else { path }
}

fn percent_decode(s string) string {
	if !s.contains('%') {
		return s
	}
	mut out := []u8{cap: s.len}
	mut i := 0
	for i < s.len {
		if s[i] == `%` && i + 2 < s.len && hex_val(s[i + 1]) >= 0 && hex_val(s[i + 2]) >= 0 {
			out << u8(hex_val(s[i + 1]) * 16 + hex_val(s[i + 2]))
			i += 3
			continue
		}
		out << s[i]
		i++
	}
	return out.bytestr()
}

fn hex_val(c u8) int {
	return match true {
		c >= `0` && c <= `9` { int(c - `0`) }
		c >= `a` && c <= `f` { int(c - `a`) + 10 }
		c >= `A` && c <= `F` { int(c - `A`) + 10 }
		else { -1 }
	}
}
