// OSC 7501, the program status protocol (rev 0.3): a program reports what it is doing, the
// terminal keeps one record per id and summarizes them. The report is untrusted: one that
// breaks a rule is dropped whole; title and msg are validated and then thrown away.
module term

import encoding.base64

pub enum ProgramState {
	idle
	done
	working
	error
	blocked
}

struct StatusRecord {
mut:
	id    string
	state ProgramState
	tick  u64
}

const max_status_records = 64

fn (mut p Parser) osc_program_status(mut s Screen, parts []string) {
	if parts.len != 2 {
		return
	}
	body := parts[1]
	if body == '?' {
		end := if p.bel { '\x07' } else { '\x1b\\' }
		s.replies << '\x1b]7501;?${end}'.bytes()
		return
	}
	if body.len > 4096 - 7 - 2 {
		return
	}
	mut state, mut id := '', ''
	mut title, mut msg := '', ''
	for pair in body.split(':') {
		eq := pair.index('=') or { continue }
		key, value := pair[..eq], pair[eq + 1..]
		if !status_key(key) || !value.bytes().all(status_value_byte(it)) {
			continue
		}
		match key {
			'state' { state = value }
			'id' { id = value }
			'title' { title = value }
			'msg' { msg = value }
			else {}
		}
	}
	st := match state {
		'idle' { ProgramState.idle }
		'working' { ProgramState.working }
		'done' { ProgramState.done }
		'blocked' { ProgramState.blocked }
		'error' { ProgramState.error }
		'clear' { ProgramState.idle }
		else { return }
	}

	if id != '' && !status_id(id) {
		return
	}
	if !status_text(title, 192) || !status_text(msg, 2048) {
		return
	}
	s.status_apply(id, st, state == 'clear')
	s.status_changed()
}

fn status_key(k string) bool {
	return k.len >= 1 && k.len <= 16 && k.bytes().all(it >= `a` && it <= `z`)
}

fn status_value_byte(b u8) bool {
	return b.is_alnum() || b in [`_`, `.`, `,`, `+`, `/`, `=`, `-`]
}

fn status_id(id string) bool {
	if id.len > 128 {
		return false
	}
	seg := id.split('/')
	if seg.len > 8 {
		return false
	}
	for v in seg {
		if v.len < 1 || v.len > 32 || !v.bytes().all(it.is_alnum() || it in [`_`, `.`, `+`, `-`]) {
			return false
		}
	}
	return true
}

// status_text: base64 of at most max bytes of UTF-8 without control characters.
fn status_text(v string, max int) bool {
	if v == '' {
		return true
	}
	if v.len > (max + 2) / 3 * 4 {
		return false
	}
	d := base64.decode(v)
	if d.len == 0 || d.len > max {
		return false
	}
	txt := d.bytestr()
	return txt.runes().all(it >= 32 && (it < 0x7f || it >= 0xa0))
}

fn (mut s Screen) status_apply(id string, st ProgramState, clear bool) {
	if clear {
		if id == '' {
			s.status.clear()
		} else {
			sub := '${id}/'
			s.status = s.status.filter(it.id != id && !it.id.starts_with(sub))
		}
		return
	}
	s.status_tick++
	for i, r in s.status {
		if r.id == id {
			s.status[i].state, s.status[i].tick = st, s.status_tick
			return
		}
	}
	if s.status.len >= max_status_records {
		mut old := 0
		for i, r in s.status {
			if r.tick < s.status[old].tick {
				old = i
			}
		}
		s.status.delete(old)
	}
	s.status << StatusRecord{id, st, s.status_tick}
}

// status_prompt is OSC 133 A: a new prompt drops working, blocked and idle.
fn (mut s Screen) status_prompt() {
	before := s.status.len
	s.status = s.status.filter(it.state in [.done, .error])
	if s.status.len != before {
		s.status_changed()
	}
}

fn (mut s Screen) status_changed() {
	mut top := ?ProgramState(none)
	for r in s.status {
		if t := top {
			if int(r.state) > int(t) {
				top = r.state
			}
		} else {
			top = r.state
		}
	}
	s.program_status = top
	s.emit(TerminalInfo{ kind: .program_status })
}
