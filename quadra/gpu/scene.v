// Scene: the primitives of one frame in paint order, plus batching.
module gpu

import quadra.prim

pub struct Scene {
pub mut:
	instances []Instance
	// revision changes with every new scene (clear). Within one revision, only the
	// instances in [dirty_start, dirty_end) changed since the last draw (patches), so a
	// device that still holds this revision uploads just that range.
	revision    u64
	dirty_start int
	dirty_end   int
	// xforms are the rigid transforms of rotated subtrees; 0 is the identity, cur the one
	// primitives are added under (core.PaintCx.push_transform).
	xforms []Rigid = [Rigid{}]
	cur    u32
mut:
	cached    []Batch
	cached_ok bool
}

pub fn (mut s Scene) clear() {
	s.instances.clear()
	s.xforms.clear()
	s.xforms << Rigid{}
	s.cur = 0
	s.revision++
	s.cached_ok = false
	s.dirty_start, s.dirty_end = 0, max_int
}

// touch marks instances [start, end) changed in place (a paint patch).
pub fn (mut s Scene) touch(start int, end int) {
	if s.dirty_start >= s.dirty_end {
		s.dirty_start, s.dirty_end = start, end
	} else {
		s.dirty_start = if start < s.dirty_start { start } else { s.dirty_start }
		s.dirty_end = if end > s.dirty_end { end } else { s.dirty_end }
	}
}

// mark_drawn records that a device has the whole scene: nothing is dirty any more.
pub fn (mut s Scene) mark_drawn() {
	s.dirty_start, s.dirty_end = 0, 0
}

// dirty_range clamps the dirty range to the instances (empty: start == end).
pub fn (s &Scene) dirty_range() (int, int) {
	end := if s.dirty_end > s.instances.len { s.instances.len } else { s.dirty_end }
	return s.dirty_start, if end < s.dirty_start {
		s.dirty_start
	} else {
		end
	}
}

pub fn (mut s Scene) quad(q Quad, mask ContentMask) {
	if q.bounds.is_empty() || (q.background.is_transparent() && q.border_widths == prim.Edges{}) {
		return
	}
	s.add(q.instance(mask))
}

pub fn (mut s Scene) shadow(sh Shadow, mask ContentMask) {
	if !sh.color.is_transparent() && !sh.bounds.is_empty() {
		s.add(sh.instance(mask))
	}
}

pub fn (mut s Scene) mono(sp Sprite, mask ContentMask) {
	s.add(sp.instance(.mono, mask))
}

pub fn (mut s Scene) poly(sp Sprite, mask ContentMask) {
	s.add(sp.instance(.poly, mask))
}

pub fn (mut s Scene) line(l LinePrim, mask ContentMask) {
	s.add(l.instance(mask))
}

// add appends an instance under the current transform.
@[inline]
fn (mut s Scene) add(i Instance) {
	s.instances << i
	if s.cur != 0 {
		s.instances[s.instances.len - 1].xform |= s.cur
	}
}

// push_xform adds a transform and makes it current; restore with set_xform.
pub fn (mut s Scene) push_xform(r Rigid) u32 {
	s.xforms << r
	s.cur = u32(s.xforms.len - 1)
	return s.cur
}

pub fn (mut s Scene) set_xform(id u32) {
	s.cur = id
}

// Batch is a contiguous run of instances drawable with one atlas page of each kind bound.
pub struct Batch {
pub:
	start     int
	count     int
	mono_page int // -1 = none needed
	poly_page int
}

// batches splits the instance list where an atlas page switch is required (cached by
// optimize; patches keep kinds and pages, so the cache stays valid).
pub fn (s &Scene) batches() []Batch {
	if s.cached_ok {
		return s.cached
	}
	return s.compute_batches()
}

fn (s &Scene) compute_batches() []Batch {
	mut out := []Batch{}
	mut start := 0
	mut mono, mut poly := -1, -1
	for i, inst in s.instances {
		need_mono := inst.kind == u32(Kind.mono)
		need_poly := inst.kind == u32(Kind.poly)
		p := int(inst.page)
		if (need_mono && mono >= 0 && mono != p) || (need_poly && poly >= 0 && poly != p) {
			out << Batch{start, i - start, mono, poly}
			start = i
			mono, poly = -1, -1
		}
		if need_mono {
			mono = p
		}
		if need_poly {
			poly = p
		}
	}
	if start < s.instances.len {
		out << Batch{start, s.instances.len - start, mono, poly}
	}
	return out
}

// optimize reorders instances to minimise batches when several atlas pages are in use,
// without changing what is visible (see order.v). It is a no-op for single-page scenes.
// optimize reorders instances into overlap layers when several atlas pages are in use;
// it reports whether it reordered (paint patches need the paint order).
pub fn (mut s Scene) optimize(mut hash SpatialHash) bool {
	mut b := s.compute_batches()
	reordered := b.len > 1
	if reordered {
		reorder_by_layers(mut s.instances, s.xforms, mut hash)
		b = s.compute_batches()
	}
	s.cached, s.cached_ok = b, true
	return reordered
}
