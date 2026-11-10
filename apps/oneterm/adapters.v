// Seams between the shell and the modules written beside it (specs/22).
module main

import quadra.prim
import store

// SavedSession is a row of the + menu's SSH block.
pub struct SavedSession {
pub:
	id    string
	title string
	color prim.Color // the session's square (default #56B6C2)
	group string     // '' = ungrouped
}

// saved_sessions lists ssh_session.json's rows in store order, ungrouped ones first.
pub fn (o &OneTerm) saved_sessions() []SavedSession {
	if o.sessions == unsafe { nil } {
		return []
	}
	mut out := []SavedSession{}
	for section in store.menu_entries(o.sessions.sessions()) {
		for row in section.rows {
			out << SavedSession{
				id:    row.id.str()
				title: row.title
				color: prim.parse_hex(row.color) or { brand_session_color }
				group: section.group
			}
		}
	}
	return out
}

const brand_session_color = prim.rgb(0x56b6c2)

pub struct NetStats {
pub:
	rx_bytes u64
	tx_bytes u64
}

// net_counters reads a Space's transport counters; none hides the indicator (every local tab).
pub fn net_counters(s &Space) ?NetStats {
	if s.session == unsafe { nil } {
		return none
	}
	rx, tx := s.session.bytes_in_out()
	return NetStats{rx, tx}
}

pub enum ProgramStatus {
	idle
	working
	blocked
	done
	error
}

// program_status is the OSC 7501 state of a Space.
pub fn program_status(s &Space) ProgramStatus {
	return s.status
}

// is_recording tells whether a Space is being logged.
pub fn is_recording(s &Space) bool {
	return s.ext != unsafe { nil } && s.ext.log.state().kind == .running
}
