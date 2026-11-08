// prim: the primitive value types every layer shares, geometry here and colors in color.v.
// Geometry units are the caller's: logical px above the paint step, device px inside gpu.
module prim

pub struct Point {
pub mut:
	x f32
	y f32
}

pub struct Size {
pub mut:
	width  f32
	height f32
}

pub struct Rect {
pub mut:
	x      f32
	y      f32
	width  f32
	height f32
}

// Edges is used for padding, margin and border widths.
pub struct Edges {
pub mut:
	top    f32
	right  f32
	bottom f32
	left   f32
}

pub struct Corners {
pub mut:
	top_left     f32
	top_right    f32
	bottom_right f32
	bottom_left  f32
}

pub fn pt(x f32, y f32) Point {
	return Point{x, y}
}

pub fn size(w f32, h f32) Size {
	return Size{w, h}
}

pub fn rect(x f32, y f32, w f32, h f32) Rect {
	return Rect{x, y, w, h}
}

pub fn edges(v f32) Edges {
	return Edges{v, v, v, v}
}

pub fn edges_xy(x f32, y f32) Edges {
	return Edges{y, x, y, x}
}

pub fn corners(v f32) Corners {
	return Corners{v, v, v, v}
}

pub fn (e Edges) horizontal() f32 {
	return e.left + e.right
}

pub fn (e Edges) vertical() f32 {
	return e.top + e.bottom
}

pub fn (e Edges) scale(s f32) Edges {
	return Edges{e.top * s, e.right * s, e.bottom * s, e.left * s}
}

pub fn (c Corners) scale(s f32) Corners {
	return Corners{c.top_left * s, c.top_right * s, c.bottom_right * s, c.bottom_left * s}
}

pub fn (c Corners) is_zero() bool {
	return c.top_left == 0 && c.top_right == 0 && c.bottom_right == 0 && c.bottom_left == 0
}

pub fn (r Rect) right() f32 {
	return r.x + r.width
}

pub fn (r Rect) bottom() f32 {
	return r.y + r.height
}

pub fn (r Rect) center() Point {
	return Point{r.x + r.width / 2, r.y + r.height / 2}
}

pub fn (r Rect) origin() Point {
	return Point{r.x, r.y}
}

pub fn (r Rect) size() Size {
	return Size{r.width, r.height}
}

pub fn (r Rect) is_empty() bool {
	return r.width <= 0 || r.height <= 0
}

pub fn (r Rect) contains(p Point) bool {
	return p.x >= r.x && p.y >= r.y && p.x < r.x + r.width && p.y < r.y + r.height
}

pub fn (a Rect) intersects(b Rect) bool {
	return a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
}

pub fn (a Rect) intersect(b Rect) Rect {
	x := max(a.x, b.x)
	y := max(a.y, b.y)
	r := min(a.right(), b.right())
	bt := min(a.bottom(), b.bottom())
	if r <= x || bt <= y {
		return Rect{x, y, 0, 0}
	}
	return Rect{x, y, r - x, bt - y}
}

pub fn (a Rect) union(b Rect) Rect {
	if a.is_empty() {
		return b
	}
	if b.is_empty() {
		return a
	}
	x := min(a.x, b.x)
	y := min(a.y, b.y)
	return Rect{x, y, max(a.right(), b.right()) - x, max(a.bottom(), b.bottom()) - y}
}

pub fn (r Rect) translate(dx f32, dy f32) Rect {
	return Rect{r.x + dx, r.y + dy, r.width, r.height}
}

pub fn (r Rect) scale(s f32) Rect {
	return Rect{r.x * s, r.y * s, r.width * s, r.height * s}
}

// inset shrinks the rect by the given edges (used for padding and borders).
pub fn (r Rect) inset(e Edges) Rect {
	return Rect{r.x + e.left, r.y + e.top, max(0, r.width - e.horizontal()), max(0,
		r.height - e.vertical())}
}

@[inline]
pub fn min(a f32, b f32) f32 {
	return if a < b { a } else { b }
}

@[inline]
pub fn max(a f32, b f32) f32 {
	return if a > b { a } else { b }
}

@[inline]
pub fn clamp(v f32, lo f32, hi f32) f32 {
	return if v < lo {
		lo
	} else if v > hi {
		hi
	} else {
		v
	}
}
