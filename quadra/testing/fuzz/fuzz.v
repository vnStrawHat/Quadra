// Deterministic fuzzing for the parsers of untrusted input (specs/11, fuzz tests). V has no
// fuzzer, so tests run a seeded generator over a corpus of valid inputs and their mutations
// inside the normal test run; QUADRA_FUZZ_ITERS raises the iteration count for long local runs.
module fuzz

import os
import time

fn C.GC_set_max_heap_size(n usize)

// max_heap_mb is a hard cap on the collector's heap of any process that imports this module:
// an input that makes a parser allocate without bound fails with out of memory at once
// instead of taking the machine's memory (Watch also checks a lower ceiling between inputs).
pub const max_heap_mb = 512

fn init() {
	C.GC_set_max_heap_size(usize(max_heap_mb) << 20)
}

// Rng is splitmix64: tiny, fast and the same sequence on every machine for a seed.
pub struct Rng {
mut:
	s u64
}

pub fn new_rng(seed u64) Rng {
	return Rng{
		s: seed
	}
}

pub fn (mut r Rng) next() u64 {
	r.s += 0x9e3779b97f4a7c15
	mut z := r.s
	z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9
	z = (z ^ (z >> 27)) * 0x94d049bb133111eb
	return z ^ (z >> 31)
}

// intn returns 0 .. n-1 (0 when n < 1).
pub fn (mut r Rng) intn(n int) int {
	if n < 1 {
		return 0
	}
	return int(r.next() % u64(n))
}

pub fn (mut r Rng) chance(percent int) bool {
	return r.intn(100) < percent
}

pub fn (mut r Rng) pick(items []string) string {
	return items[r.intn(items.len)]
}

pub fn (mut r Rng) bytes(n int) []u8 {
	mut b := []u8{len: n}
	for i in 0 .. n {
		b[i] = u8(r.next())
	}
	return b
}

// iters is the iteration count of a fuzz test: base, or QUADRA_FUZZ_ITERS when set.
pub fn iters(base int) int {
	n := os.getenv('QUADRA_FUZZ_ITERS').int()
	return if n > 0 { n } else { base }
}

// Integers at the edges of the usual widths, as text and as big-endian u32 fields.
pub const boundary_texts = ['0', '-1', '1', '65535', '65536', '2147483647', '2147483648',
	'4294967295', '4294967296', '-2147483648', '99999999999999999999', '1e308', '-0', 'NaN']
pub const boundary_u32 = [u32(0), 1, 0x7f, 0x80, 0xff, 0xffff, 0x10000, 0x7fffffff, 0x80000000,
	0xfffffffe, 0xffffffff]

// Byte runs that break UTF-8 decoders: stray continuations, overlongs, surrogates, cut sequences.
pub const bad_utf8 = [[u8(0xff)], [u8(0x80)], [u8(0xc0), 0xaf],
	[u8(0xe2), 0x82], [u8(0xed), 0xa0, 0x80], [u8(0xf4), 0x90, 0x80, 0x80],
	[u8(0xf0), 0x9f, 0x98], [u8(0xfe), 0xfe, 0xff]]

// mutate returns a mutated copy of src: one to four of byte flips, truncation, duplication of
// a chunk (repeated up to hundreds of times: deep nesting from an opener), splicing with
// another corpus entry, boundary integers, huge length fields and invalid UTF-8.
pub fn (mut r Rng) mutate(src []u8, corpus [][]u8) []u8 {
	mut b := src.clone()
	for _ in 0 .. 1 + r.intn(4) {
		b = r.mutate_once(b, corpus)
	}
	return b
}

fn (mut r Rng) mutate_once(src []u8, corpus [][]u8) []u8 {
	mut b := src.clone()
	at := r.intn(b.len + 1)
	match r.intn(9) {
		0 { // flip bits in a few bytes
			for _ in 0 .. 1 + r.intn(4) {
				if b.len > 0 {
					b[r.intn(b.len)] ^= u8(1 << r.intn(8))
				}
			}
		}
		1 { // random bytes overwrite
			for _ in 0 .. 1 + r.intn(8) {
				if b.len > 0 {
					b[r.intn(b.len)] = u8(r.next())
				}
			}
		}
		2 { // truncate
			b = b[..at].clone()
		}
		3 { // duplicate a chunk, sometimes many times
			n := 1 + r.intn(16)
			if at + n <= b.len {
				chunk := b[at..at + n].clone()
				times := if r.chance(20) { 50 + r.intn(500) } else { 1 + r.intn(4) }
				mut out := b[..at].clone()
				for _ in 0 .. times {
					out << chunk
				}
				out << b[at..]
				b = out.clone()
			}
		}
		4 { // splice: a prefix of this one and a suffix of another
			if corpus.len > 0 {
				o := corpus[r.intn(corpus.len)]
				mut out := b[..at].clone()
				out << o[r.intn(o.len + 1)..]
				b = out.clone()
			}
		}
		5 { // boundary integer as text
			mut out := b[..at].clone()
			out << r.pick(boundary_texts).bytes()
			out << b[at..]
			b = out.clone()
		}
		6 { // boundary or huge length as a big-endian u32, overwriting or inserting
			v := boundary_u32[r.intn(boundary_u32.len)]
			field := [u8(v >> 24), u8(v >> 16), u8(v >> 8), u8(v)]
			if r.chance(50) && at + 4 <= b.len {
				for k in 0 .. 4 {
					b[at + k] = field[k]
				}
			} else {
				mut out := b[..at].clone()
				out << field
				out << b[at..]
				b = out.clone()
			}
		}
		7 { // invalid UTF-8
			mut out := b[..at].clone()
			out << bad_utf8[r.intn(bad_utf8.len)]
			out << b[at..]
			b = out.clone()
		}
		else { // delete a chunk
			n := 1 + r.intn(8)
			if at + n <= b.len {
				mut out := b[..at].clone()
				out << b[at + n..]
				b = out.clone()
			}
		}
	}

	return b
}

// Watch times each input of a fuzz run (a hang shows as one slow input) and checks the heap
// does not keep growing: the collected heap at the end against the one halfway (the first half
// is warm-up, where the collector's heap grows legitimately), and caps it: an input that makes
// a parser allocate in proportion to a length or count it read fails at once, instead of
// taking the machine's memory (the collector aborts with "Too many heap sections").
pub struct Watch {
	name  string
	total int
	sw    time.StopWatch
pub mut:
	ceiling_mb int = default_ceiling_mb // the collector's heap may never pass this
mut:
	heap_mid  usize
	heap_peak usize
	lap       time.StopWatch
	worst     i64 // ms
	worst_at  int
	inputs    int
	max_input int
}

pub fn watch(name string, total int) Watch {
	return Watch{
		name:  name
		total: total
		sw:    time.new_stopwatch()
		lap:   time.new_stopwatch()
	}
}

pub const default_ceiling_mb = 256

// check_ceiling fails when the collector's heap is over the ceiling (done calls it).
pub fn (mut w Watch) check_ceiling(i int) {
	heap := gc_heap_usage().heap_size
	if heap > w.heap_peak {
		w.heap_peak = heap
	}
	mb := int(heap >> 20)
	assert mb <= w.ceiling_mb, '${w.name}: heap at ${mb} MB after input ${i} (ceiling ${w.ceiling_mb} MB)'
}

// start marks the start of one input.
pub fn (mut w Watch) start() {
	w.lap.restart()
}

// done records input i of len bytes and fails when it took longer than limit_ms or the heap
// passed the ceiling.
pub fn (mut w Watch) done(i int, len int, limit_ms i64) {
	ms := w.lap.elapsed().milliseconds()
	w.inputs++
	if w.inputs == w.total / 2 {
		gc_collect()
		w.heap_mid = gc_heap_usage().heap_size
	}
	if len > w.max_input {
		w.max_input = len
	}
	if ms > w.worst {
		w.worst, w.worst_at = ms, i
	}
	assert ms <= limit_ms, '${w.name}: input ${i} (${len} bytes) took ${ms} ms'
	w.check_ceiling(i)
}

// finish fails when the heap grew by more than grow_mb over the second half, and prints a
// summary.
pub fn (mut w Watch) finish(grow_mb int) {
	gc_collect()
	heap := gc_heap_usage().heap_size
	grown := if w.heap_mid > 0 && heap > w.heap_mid { int((heap - w.heap_mid) >> 20) } else { 0 }
	println('fuzz ${w.name}: ${w.inputs} inputs in ${w.sw.elapsed().milliseconds()} ms, slowest ${w.worst} ms (#${w.worst_at}), largest ${w.max_input} B, heap +${grown} MB, peak ${w.heap_peak >> 20} MB')
	assert grown <= grow_mb, '${w.name}: heap grew ${grown} MB'
}
