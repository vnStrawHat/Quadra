// The in-session command history (`memory` source): one bounded ring per shell family, every
// entry with frecency bookkeeping. Lines are recorded already redacted. RAM only (specs/24).
module completion

pub const default_history_capacity = 500

// FrecencyRecord is a recorded command line and its use statistics.
pub struct FrecencyRecord {
pub mut:
	line         string
	use_count    u32
	last_used_ms u64 // supplied by the caller: the engine stays clock-free
}

// frecency is use_count * recency_decay(age).
pub fn (r FrecencyRecord) frecency(now_ms u64) f32 {
	age := if now_ms > r.last_used_ms { now_ms - r.last_used_ms } else { u64(0) }
	age_hours := f64(age) / 3_600_000.0
	return f32(f64(r.use_count) / (1.0 + age_hours))
}

struct CommandRing {
mut:
	entries  []FrecencyRecord
	capacity int
}

fn (mut ring CommandRing) record(line string, now_ms u64) {
	if ring.capacity == 0 {
		return
	}
	for mut rec in ring.entries {
		if rec.line == line {
			if rec.use_count != max_u32 {
				rec.use_count++
			}
			rec.last_used_ms = now_ms
			return
		}
	}
	ring.entries << FrecencyRecord{line, 1, now_ms}
	ring.evict()
}

fn (mut ring CommandRing) evict() {
	for ring.entries.len > ring.capacity {
		mut idx := 0
		for i, rec in ring.entries {
			if rec.last_used_ms < ring.entries[idx].last_used_ms {
				idx = i
			}
		}
		ring.entries.delete(idx)
	}
}

// History is the global, cross-tab store.
pub struct History {
mut:
	rings    []CommandRing
	capacity int
}

// new_history returns a store with the given per-family capacity.
pub fn new_history(capacity int) History {
	mut h := History{
		capacity: capacity
	}
	for _ in 0 .. 3 {
		h.rings << CommandRing{
			capacity: capacity
		}
	}
	return h
}

// record stores an already-redacted command line.
pub fn (mut h History) record(family ShellFamily, redacted_line string, now_ms u64) {
	line := redacted_line.trim_space()
	if line.len == 0 || h.capacity == 0 {
		return
	}
	h.rings[int(family)].record(line, now_ms)
}

// entries returns the records of a family.
pub fn (h &History) entries(family ShellFamily) []FrecencyRecord {
	return h.rings[int(family)].entries
}

// set_capacity changes the per-family capacity; 0 disables and clears the store.
pub fn (mut h History) set_capacity(n int) {
	h.capacity = n
	if n == 0 {
		h.clear()
		return
	}
	for mut ring in h.rings {
		ring.capacity = n
		ring.evict()
	}
}

pub fn (h &History) capacity() int {
	return h.capacity
}

// clear forgets everything recorded (the "Clear history" action).
pub fn (mut h History) clear() {
	for mut ring in h.rings {
		ring.entries.clear()
		ring.capacity = h.capacity
	}
}

pub fn (h &History) is_empty() bool {
	for ring in h.rings {
		if ring.entries.len > 0 {
			return false
		}
	}
	return true
}
