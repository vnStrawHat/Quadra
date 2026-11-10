// The layout tree: a flat node arena. Ids are indices and stay valid until clear().
module layout

import quadra.prim

pub type NodeId = int

// Sentinel sizes. `unknown` means "not yet determined"; `unbounded` means "no limit".
pub const unknown = f32(-1)
pub const unbounded = f32(1.0e30)

// MeasureFn reports the intrinsic content size of a leaf (text, images).
// known_* are fixed content sizes or `unknown`; avail_* are limits or `unbounded`.
// ctx is the element's checked holder (prim.held, specs/14).
pub type MeasureFn = fn (ctx prim.Holder, known_w f32, known_h f32, avail_w f32, avail_h f32) prim.Size

// SameFn reports whether two leaf contexts measure alike for every request (same text and
// font): incremental layout then takes the old leaf's layout over without measuring.
pub type SameFn = fn (old prim.Holder, fresh prim.Holder) bool

struct CacheEntry {
	kw  f32
	kh  f32
	aw  f32
	ah  f32
	out prim.Size
}

struct Node {
mut:
	style    Style
	children []NodeId
	measure  MeasureFn = unsafe { nil }
	ctx      prim.Holder
	rect     prim.Rect // border box, relative to the parent's border box
	content  prim.Size // extent of the children, for scrolling
	cache    [cache_slots]CacheEntry
	// incremental layout (relayout.v)
	parent NodeId = -1 // -2 - id: replace() put node id in its place
	owner  u32
	valid  u8   // bit i: cache[i] holds a result
	next   u8   // the entry the next result goes to
	pslot  u8   // the cache entry of the last perform request
	done   bool // performed at that request: the rects below are that layout's
	probed bool // measured since that perform (a leaf's element may hold the probe's wrap)
	lost   bool // the cache overwrote an answer: what the parent was told is not all known
}

// cache_slots measure results are kept per node: flex asks a child for its base size, its
// cross size at the final main size, then lays it out; nested containers repeat that.
const cache_slots = 4

pub struct Tree {
mut:
	nodes []Node
	// flex scratch, one per nesting depth of layout_flex (flex.v): reused, never reallocated
	flex_items [][]FlexItem
	flex_lines [][]FlexLine
	depth      int
	recycle    []NodeId   // nodes to fill again, next last (relayout.v)
	sames      []SamePair // the SameFn of each measure function given one (leaf_same)
	same_last  MeasureFn = unsafe { nil } // the last one registered
	avail      prim.Size // the last compute's available size
pub mut:
	measure_calls int // statistics: number of layout_node evaluations not served by the cache
	owner         u32 // stamped on new nodes: the view being laid out (relayout.v)
	climb_work    int // children of the ancestors replace() laid out again
}

pub fn (mut t Tree) clear() {
	t.nodes.clear()
	t.recycle.clear()
	t.measure_calls = 0
}

pub fn (t &Tree) len() int {
	return t.nodes.len
}

// truncate drops the nodes added after the first n (scratch layouts done during prepaint:
// virtual list rows, overlays). Ids below n stay valid.
pub fn (mut t Tree) truncate(n int) {
	if n < t.nodes.len {
		t.nodes.trim(n)
	}
}

pub fn (t &Tree) style(id NodeId) &Style {
	return &t.nodes[id].style
}

pub fn (t &Tree) children(id NodeId) []NodeId {
	return t.nodes[id].children
}

pub fn (t &Tree) rect(id NodeId) prim.Rect {
	return t.nodes[id].rect
}

pub fn (t &Tree) content_size(id NodeId) prim.Size {
	return t.nodes[id].content
}

// compute lays out the tree under root for the given available size.
// The root takes its style size, or fills the available space when auto.
pub fn (mut t Tree) compute(root NodeId, available prim.Size) {
	for mut n in t.nodes { // from scratch: every result is forgotten
		n.valid, n.next, n.done, n.probed, n.lost = 0, 0, false, false, false
	}
	t.avail = available
	t.layout_root(root, available)
}

// compute_new is compute for a tree built since clear(): nothing to forget.
pub fn (mut t Tree) compute_new(root NodeId, available prim.Size) {
	t.avail = available
	t.layout_root(root, available)
}

// layout_root lays out a root filling the available size (compute, replace).
fn (mut t Tree) layout_root(root NodeId, available prim.Size) {
	s := t.nodes[root].style
	mut w := s.width.resolve(available.width)
	mut h := s.height.resolve(available.height)
	if w == unknown {
		w = available.width
	}
	if h == unknown {
		h = available.height
	}
	out := t.layout_node(root, w, h, available.width, available.height, true)
	t.nodes[root].rect = prim.rect(0, 0, out.width, out.height)
}

// compute_fit lays out a root that sizes to its content (popovers, menus, tooltips),
// limited by the available size.
pub fn (mut t Tree) compute_fit(root NodeId, available prim.Size) {
	s := t.nodes[root].style
	w := s.width.resolve(available.width)
	h := s.height.resolve(available.height)
	sz := t.layout_node(root, w, h, available.width, available.height, false)
	out := t.layout_node(root, sz.width, sz.height, sz.width, sz.height, true)
	t.nodes[root].rect = prim.rect(0, 0, out.width, out.height)
}

// compute_width lays out a freshly built subtree at a fixed width and its content height,
// which it returns (variable-height list rows).
pub fn (mut t Tree) compute_width(root NodeId, w f32) f32 {
	sz := t.layout_node(root, w, unknown, w, unknown, false)
	t.layout_node(root, w, sz.height, w, sz.height, true)
	t.nodes[root].rect = prim.rect(0, 0, w, sz.height)
	return sz.height
}

// compute_fixed lays out a freshly built subtree at an exact size (virtual list rows).
// Fresh nodes have empty caches, so nothing needs resetting.
pub fn (mut t Tree) compute_fixed(root NodeId, w f32, h f32) {
	t.layout_node(root, w, h, w, h, true)
	t.nodes[root].rect = prim.rect(0, 0, w, h)
}

@[inline]
fn has(mask u8, i int) bool {
	return mask & (u8(1) << i) != 0
}

fn (t &Tree) cached(id NodeId, kw f32, kh f32, aw f32, ah f32) ?prim.Size {
	n := unsafe { &t.nodes[id] }
	for i in 0 .. cache_slots {
		e := unsafe { &n.cache[i] }
		if has(n.valid, i) && e.kw == kw && e.kh == kh && e.aw == aw && e.ah == ah {
			return e.out
		}
	}
	return none
}

// store records a result; a perform's becomes the node's last perform, whose entry is never
// the one replaced. Only a perform can find its request there: a measure stores after a miss.
fn (mut t Tree) store(id NodeId, kw f32, kh f32, aw f32, ah f32, out prim.Size, perform bool) {
	mut n := &t.nodes[id]
	n.probed = !perform
	if perform {
		for i in 0 .. cache_slots {
			e := unsafe { &n.cache[i] }
			if has(n.valid, i) && e.kw == kw && e.kh == kh && e.aw == aw && e.ah == ah {
				n.cache[i] = CacheEntry{kw, kh, aw, ah, out}
				n.pslot, n.done = u8(i), true
				return
			}
		}
	}
	if n.done && n.next == n.pslot {
		n.next = u8((int(n.next) + 1) % cache_slots)
	}
	i := int(n.next)
	if has(n.valid, i) {
		n.lost = true
	}
	n.cache[i] = CacheEntry{kw, kh, aw, ah, out}
	n.valid |= u8(1) << i
	n.next = u8((i + 1) % cache_slots)
	if perform {
		n.pslot, n.done = u8(i), true
	}
}

// layout_node returns the border-box size of a node. kw/kh are sizes fixed by the
// parent (or unknown); aw/ah are the space available to it. With perform=true the
// children are also positioned; measuring calls (perform=false) never move anything.
fn (mut t Tree) layout_node(id NodeId, kw_ f32, kh_ f32, aw_ f32, ah_ f32, perform bool) prim.Size {
	mut kw, mut kh := kw_, kh_
	// A size fixed by the parent makes the space available on that axis irrelevant, so
	// requests that differ only there share one cache entry.
	aw := if kw_ != unknown { kw_ } else { aw_ }
	ah := if kh_ != unknown { kh_ } else { ah_ }
	if !perform {
		if out := t.cached(id, kw_, kh_, aw, ah) {
			return out
		}
	} else {
		// Laid out at this request before and nothing below changed since: every rect under
		// it still holds (a position-only change moves it, its parent sets its rect).
		n := unsafe { &t.nodes[id] }
		if n.done && !n.probed {
			e := n.cache[n.pslot]
			if e.kw == kw_ && e.kh == kh_ && e.aw == aw && e.ah == ah {
				return e.out
			}
		}
	}
	t.measure_calls++
	s := unsafe { &t.nodes[id].style } // no copy: Style is large and this is the hottest path
	// Explicit px sizes are definite no matter who asks.
	if kw == unknown && s.width.unit == .px {
		kw = clamp_axis(s.width.value, s.min_width.resolve(aw), s.max_width.resolve(aw))
	}
	if kh == unknown && s.height.unit == .px {
		kh = clamp_axis(s.height.value, s.min_height.resolve(ah), s.max_height.resolve(ah))
	}
	mut out := prim.Size{}
	if t.nodes[id].measure != unsafe { nil } {
		out = t.measure_leaf(id, s, kw, kh, aw, ah)
	} else if s.display == .grid {
		out = t.layout_grid(id, s, kw, kh, aw, ah, perform)
	} else {
		out = t.layout_flex(id, s, kw, kh, aw, ah, perform)
	}
	t.store(id, kw_, kh_, aw, ah, out, perform)
	return out
}

fn (mut t Tree) measure_leaf(id NodeId, s &Style, kw f32, kh f32, aw f32, ah f32) prim.Size {
	f := s.frame()
	inner := fn (v f32, sub f32) f32 {
		return if v == unknown {
			unknown
		} else if v >= unbounded {
			unbounded
		} else {
			prim.max(0, v - sub)
		}
	}
	n := t.nodes[id]
	m := n.measure(n.ctx, inner(kw, f.horizontal()), inner(kh, f.vertical()), inner(aw,
		f.horizontal()), inner(ah, f.vertical()))
	w := if kw != unknown { kw } else { m.width + f.horizontal() }
	h := if kh != unknown { kh } else { m.height + f.vertical() }
	return clamp_size(s, w, h, kw != unknown, kh != unknown, aw, ah)
}

// clamp_size applies the node's own min/max to sizes the parent did not fix.
fn clamp_size(s &Style, w f32, h f32, fixed_w bool, fixed_h bool, aw f32, ah f32) prim.Size {
	mut rw, mut rh := w, h
	if !fixed_w {
		rw = clamp_axis(rw, s.min_width.resolve(aw), s.max_width.resolve(aw))
	}
	if !fixed_h {
		rh = clamp_axis(rh, s.min_height.resolve(ah), s.max_height.resolve(ah))
	}
	return prim.size(rw, rh)
}

fn clamp_axis(v f32, lo f32, hi f32) f32 {
	mut r := v
	if hi != unknown && r > hi {
		r = hi
	}
	if lo != unknown && r < lo {
		r = lo
	}
	return r
}
