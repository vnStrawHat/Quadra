// Box drawing built from arms: lines, corners, tees, crosses, half lines, double lines and
// dashes (specs/17). `arms` packs [up, down, left, right] two bits each: Weight below.
module term

enum Weight {
	none_
	light
	heavy
	double
}

fn (w Weight) stroke(t StrokeThickness) int {
	return match w {
		.light { t.light }
		.heavy { t.heavy }
		.double { t.rail }
		.none_ { 0 }
	}
}

enum Dir {
	up
	down
	left
	right
}

fn (d Dir) along_axis() Axis {
	return if d == .up || d == .down { Axis.y } else { Axis.x }
}

fn (d Dir) opposite() Dir {
	return match d {
		.up { Dir.down }
		.down { Dir.up }
		.left { Dir.right }
		.right { Dir.left }
	}
}

// perpendicular is the arm on a side (-1 or +1) of this arm's cross axis.
fn (d Dir) perpendicular(side int) Dir {
	if d.along_axis() == .x {
		return if side < 0 { Dir.up } else { Dir.down }
	}
	return if side < 0 { Dir.left } else { Dir.right }
}

// near_side is the rail of a perpendicular double line this arm reaches first.
fn (d Dir) near_side() int {
	return if d == .down || d == .right { 1 } else { -1 }
}

fn (d Dir) edge_of(i Interval) int {
	return if d == .down || d == .right { i.lo } else { i.hi }
}

struct Mirror {
	x bool
	y bool
}

fn mirror_of(code u16) Mirror {
	return Mirror{code & 0x100 != 0, code & 0x200 != 0}
}

fn arms_of(code u16) u8 {
	return u8(code & 0xff)
}

// mirror_arms reflects an arm set: a permutation of its slots.
fn mirror_arms(arms u8, m Mirror) u8 {
	up, down, left, right := arm_slot(arms, 0), arm_slot(arms, 1), arm_slot(arms, 2), arm_slot(arms, 3)
	mut a, mut b, mut c, mut d := up, down, left, right
	if m.y {
		a, b = down, up
	}
	if m.x {
		c, d = right, left
	}
	return u8(a) | u8(b) << 2 | u8(c) << 4 | u8(d) << 6
}

@[inline]
fn arm_slot(arms u8, slot int) u8 {
	return (arms >> (2 * slot)) & 3
}

@[inline]
fn arm(arms u8, d Dir) Weight {
	return unsafe { Weight(arm_slot(arms, int(d))) }
}

fn weight_max(a Weight, b Weight) Weight {
	return if int(a) >= int(b) { a } else { b }
}

// arm_boundary is where an arm stops meeting the rest of the glyph; `side` is the rail of this
// arm when it is double (0 for a single arm).
fn arm_boundary(d Dir, side int, own Weight, arms u8, g &Geometry) int {
	along := d.along_axis()
	perp_minus, perp_plus := arm(arms, d.perpendicular(-1)), arm(arms, d.perpendicular(1))
	if perp_minus != .double && perp_plus != .double {
		mut weight := weight_max(perp_minus, perp_plus)
		if weight == .none_ {
			weight = own
		}
		return d.edge_of(g.joint(along, weight.stroke(g.t)))
	}
	mut use_near := false
	if side < 0 {
		use_near = perp_minus != .none_
	} else if side > 0 {
		use_near = perp_plus != .none_
	} else {
		use_near = arm(arms, d.opposite()) == .none_ && perp_minus != .none_ && perp_plus != .none_
	}
	rail_side := if use_near { d.near_side() } else { -d.near_side() }
	return d.edge_of(g.rail(along, rail_side))
}

fn arm_along(d Dir, side int, own Weight, arms u8, g &Geometry) Interval {
	size := g.size(d.along_axis())
	cut := arm_boundary(d, side, own, arms, g)
	return if d == .down || d == .right { Interval{cut, size} } else { Interval{0, cut} }
}

struct Piece {
	axis Axis
mut:
	along Interval
	cross Interval
}

fn add_piece(mut pieces []Piece, p Piece) {
	if p.along.empty() || p.cross.empty() {
		return
	}
	for i, e in pieces {
		if e.axis == p.axis && e.cross == p.cross && e.along.touches(p.along) {
			pieces[i].along = e.along.join(p.along)
			return
		}
	}
	pieces << p
}

const all_dirs = [Dir.up, .down, .left, .right]

fn build_arms(arms_ u8, m Mirror, g &Geometry, mut out []ShapeRect) {
	arms := mirror_arms(arms_, m)
	mut pieces := []Piece{cap: 8}
	for d in all_dirs {
		weight := arm(arms, d)
		if weight == .none_ {
			continue
		}
		along_axis := d.along_axis()
		cross_axis := along_axis.other()
		if weight == .double {
			add_piece(mut pieces, Piece{along_axis, arm_along(d, -1, weight, arms, g), g.rail(cross_axis,
				-1)})
			add_piece(mut pieces, Piece{along_axis, arm_along(d, 1, weight, arms, g), g.rail(cross_axis,
				1)})
		} else {
			add_piece(mut pieces, Piece{along_axis, arm_along(d, 0, weight, arms, g), g.joint(cross_axis,
				weight.stroke(g.t))})
		}
	}
	for p in pieces {
		if p.axis == .x {
			push_rect(mut out, p.along, p.cross)
		} else {
			push_rect(mut out, p.cross, p.along)
		}
	}
}

struct DashDef {
	count  int
	weight Weight
	axis   Axis
}

fn dash_def(code u32) ?DashDef {
	return match code {
		0x2504 { DashDef{3, .light, .x} }
		0x2505 { DashDef{3, .heavy, .x} }
		0x2506 { DashDef{3, .light, .y} }
		0x2507 { DashDef{3, .heavy, .y} }
		0x2508 { DashDef{4, .light, .x} }
		0x2509 { DashDef{4, .heavy, .x} }
		0x250a { DashDef{4, .light, .y} }
		0x250b { DashDef{4, .heavy, .y} }
		0x254c { DashDef{2, .light, .x} }
		0x254d { DashDef{2, .heavy, .x} }
		0x254e { DashDef{2, .light, .y} }
		0x254f { DashDef{2, .heavy, .y} }
		else { none }
	}
}

fn build_dashes(dash DashDef, g &Geometry, mut out []ShapeRect) {
	axis, count := dash.axis, dash.count
	center, size := g.center(axis), g.size(axis)
	cross := g.joint(axis.other(), dash.weight.stroke(g.t))
	if cross.empty() {
		return
	}
	pitch := f32(size) / f32(count)
	gap := f32(g.t.dash_gap)
	mut half := if (pitch - gap) / 2 > 0.5 { (pitch - gap) / 2 } else { f32(0.5) }
	mut segments := []Interval{len: count}
	for {
		for k in 0 .. count {
			c := center + (f32(k) + 0.5 - f32(count) / 2) * pitch
			segments[k] = snap_interval(c, half, size, .nearest)
		}
		mut overlaps := false
		for k in 1 .. count {
			if segments[k].lo < segments[k - 1].hi {
				overlaps = true
			}
		}
		if !overlaps || half <= 0.5 {
			break
		}
		half -= 0.5 // shrinking every segment alike keeps the set symmetric
	}
	mut k := 0
	for k < count {
		mut merged := segments[k]
		for k + 1 < count && segments[k + 1].lo < merged.hi {
			merged = merged.join(segments[k + 1])
			k++
		}
		k++
		if axis == .x {
			push_rect(mut out, merged, cross)
		} else {
			push_rect(mut out, cross, merged)
		}
	}
}
