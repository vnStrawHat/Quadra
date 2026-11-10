// Synchronized updates (mode 2026): while one is open the parser holds the bytes back, so the
// screen stays at the last complete frame (specs/17). Linear in the bytes fed: each byte is
// searched for the end of the update once, and the hold is capped.
module term

// max_sync_hold caps the bytes an open update holds back: a program that never closes it
// is parsed at once past this, as when the view's timeout ends it.
pub const max_sync_hold = 4 << 20

const sync_end = '\x1b[?2026l'

// hold_rest is called with an update open at data[i]: true when data holds no end of it, so
// the rest is held back; parsing goes on otherwise (the end comes in this same feed).
fn (mut p Parser) hold_rest(data []u8, i int) bool {
	if i <= p.sync_ok {
		return false
	}
	at := index_from(data, i, sync_end)
	if at >= 0 {
		p.sync_ok = at
		return false
	}
	p.held = data[i..].clone()
	p.held_on = true
	return true
}

// release_held adds data to the held bytes and parses them when the end of the update has
// arrived or the hold is over max_sync_hold. Only the new bytes are searched (from a few
// before them: the end may be split across feeds).
fn (mut p Parser) release_held(mut s Screen, data []u8) {
	from := if p.held.len > sync_end.len { p.held.len - sync_end.len } else { 0 }
	p.held << data
	if p.held.len > max_sync_hold {
		s.sync = false
	} else if index_from(p.held, from, sync_end) < 0 {
		return
	}
	buf := p.held
	p.held = []u8{}
	p.held_on = false
	p.feed(mut s, buf)
}

// index_from is the index of pat in data at or after from, -1 when absent.
fn index_from(data []u8, from int, pat string) int {
	if pat.len == 0 {
		return -1
	}
	last := data.len - pat.len
	first := pat[0]
	mut i := if from < 0 { 0 } else { from }
	for i <= last {
		if data[i] == first {
			mut k := 1
			for k < pat.len && data[i + k] == pat[k] {
				k++
			}
			if k == pat.len {
				return i
			}
		}
		i++
	}
	return -1
}
