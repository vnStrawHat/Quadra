// Heights: the row heights of a variable-height list in a Fenwick tree, so offset-to-row
// and row-to-offset lookups are O(log n) for millions of rows (specs/08). Rows not yet
// measured carry the estimate.
module ui

pub struct Heights {
mut:
	h    []f32 // per row
	tree []f64 // 1-based Fenwick sums of h
	step int   // highest power of two <= h.len, for find
}

// resize keeps the known heights of rows that remain and gives new rows the estimate.
// ponytail: rebuilds the tree, O(n) per count change; a feed growing every frame at 100k+
// rows wants an append path that extends the tree instead.
fn (mut hs Heights) resize(count int, est f32) {
	old := hs.h.len
	if count < old {
		hs.h.trim(count)
	} else {
		for _ in old .. count {
			hs.h << est
		}
	}
	hs.tree = []f64{len: count + 1}
	for i in 1 .. count + 1 {
		hs.tree[i] += hs.h[i - 1]
		j := i + (i & -i)
		if j <= count {
			hs.tree[j] += hs.tree[i]
		}
	}
	hs.step = 1
	for hs.step * 2 <= count {
		hs.step *= 2
	}
}

fn (hs &Heights) height(i int) f32 {
	return hs.h[i]
}

// prefix is the offset of row i: the sum of the rows above it.
fn (hs &Heights) prefix(i int) f64 {
	mut s := f64(0)
	mut j := i
	for j > 0 {
		s += hs.tree[j]
		j -= j & -j
	}
	return s
}

fn (hs &Heights) total() f64 {
	return hs.prefix(hs.h.len)
}

fn (mut hs Heights) set(i int, v f32) {
	d := f64(v) - f64(hs.h[i])
	if d == 0 {
		return
	}
	hs.h[i] = v
	mut j := i + 1
	for j < hs.tree.len {
		hs.tree[j] += d
		j += j & -j
	}
}

// find returns the row that contains offset y (the last row past the end).
fn (hs &Heights) find(y f64) int {
	mut pos := 0
	mut rem := y
	mut step := hs.step
	for step > 0 {
		if pos + step < hs.tree.len && hs.tree[pos + step] <= rem {
			pos += step
			rem -= hs.tree[pos]
		}
		step /= 2
	}
	return if pos >= hs.h.len { hs.h.len - 1 } else { pos }
}
