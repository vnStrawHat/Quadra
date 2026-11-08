// A flat text-keyed cache with frame generations: one map lookup per hit and no copying
// between generations. Entries not used for `keep` frames are swept every `cache_sweep_frames`
// frames. Hash collisions are detected (the key is stored) and treated as misses.
module text

import hash.fnv1a

pub const cache_sweep_frames = u32(32)
const keep = u32(2)

@[heap]
struct Slot[T] {
	text string
	k1   u64
	k2   u64
mut:
	gen u32
	val T
}

struct Cache[T] {
mut:
	m   map[u64]&Slot[T]
	gen u32
}

@[inline]
fn slot_hash(s string, k1 u64, k2 u64) u64 {
	return fnv1a.sum64_string(s) ^ (k1 * 0x9E3779B97F4A7C15) ^ ((k2 + 1) * 0xC2B2AE3D27D4EB4F)
}

fn (mut c Cache[T]) get(s string, k1 u64, k2 u64) ?T {
	if mut e := c.m[slot_hash(s, k1, k2)] {
		if e.k1 == k1 && e.k2 == k2 && e.text == s {
			e.gen = c.gen
			return e.val
		}
	}
	return none
}

fn (mut c Cache[T]) put(s string, k1 u64, k2 u64, v T) {
	c.m[slot_hash(s, k1, k2)] = &Slot[T]{
		text: s
		k1:   k1
		k2:   k2
		gen:  c.gen
		val:  v
	}
}

fn (mut c Cache[T]) end_frame() {
	c.gen++
	if c.gen % cache_sweep_frames != 0 {
		return
	}
	mut gone := []u64{}
	for h, e in c.m {
		if c.gen - e.gen > keep {
			gone << h
		}
	}
	for h in gone {
		c.m.delete(h)
	}
}
