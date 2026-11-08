// Rigid transforms (rotation plus translation, device px) for rotated subtrees: the
// scene keeps them in `xforms`, and instances name theirs (specs/03, "Rotation").
module gpu

import math
import quadra.prim

// Rigid maps p to R(p) + t, R the rotation with cosine c and sine s. It mirrors one
// float4 of the shader's xforms buffer.
pub struct Rigid {
pub:
	c  f32 = 1
	s  f32
	tx f32
	ty f32
}

// rotation_about rotates by deg degrees (clockwise on screen) about (px, py).
pub fn rotation_about(deg f32, px f32, py f32) Rigid {
	rad := f64(deg) * math.pi / 180
	c, s := f32(math.cos(rad)), f32(math.sin(rad))
	return Rigid{c, s, px - (c * px - s * py), py - (s * px + c * py)}
}

pub fn (r Rigid) apply(x f32, y f32) (f32, f32) {
	return r.c * x - r.s * y + r.tx, r.s * x + r.c * y + r.ty
}

// then is r applied after inner: p -> r(inner(p)).
pub fn (r Rigid) then(inner Rigid) Rigid {
	tx, ty := r.apply(inner.tx, inner.ty)
	return Rigid{r.c * inner.c - r.s * inner.s, r.s * inner.c + r.c * inner.s, tx, ty}
}

pub fn (r Rigid) inverse() Rigid {
	return Rigid{r.c, -r.s, -(r.c * r.tx + r.s * r.ty), r.s * r.tx - r.c * r.ty}
}

// bbox is the axis-aligned box around the mapped rect.
pub fn (r Rigid) bbox(b prim.Rect) prim.Rect {
	if r.s == 0 && r.c == 1 {
		return prim.rect(b.x + r.tx, b.y + r.ty, b.width, b.height)
	}
	mut x0, mut y0 := f32(math.max_f32), f32(math.max_f32)
	mut x1, mut y1 := -x0, -y0
	for p in [[b.x, b.y]!, [b.x + b.width, b.y]!, [b.x, b.y + b.height]!,
		[b.x + b.width, b.y + b.height]!] {
		x, y := r.apply(p[0], p[1])
		x0, y0 = f32(math.min(x0, x)), f32(math.min(y0, y))
		x1, y1 = f32(math.max(x1, x)), f32(math.max(y1, y))
	}
	return prim.rect(x0, y0, x1 - x0, y1 - y0)
}
