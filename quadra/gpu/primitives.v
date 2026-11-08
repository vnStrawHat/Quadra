// Scene primitives. Every primitive is encoded as one fixed-size Instance (an
// "uber" instance) so a whole scene draws in paint order with as few draw calls as
// there are atlas-page switches. Coordinates are device px.
module gpu

import quadra.prim

pub enum Kind {
	quad   = 0
	shadow = 1
	mono   = 2 // glyphs and icons: coverage from the mono atlas, tinted by color
	poly   = 3 // images, emoji: RGBA from the poly atlas
	line   = 4 // underline, wavy underline, chart line column, stroke segment (stroke.v)
}

// Instance mirrors `struct Instance` in gpu/d3d11/shaders/prims.hlsl (144 bytes).
pub struct Instance {
pub mut:
	bounds [4]f32 // x, y, width, height
	clip   [4]f32 // content mask x, y, width, height
	color  [4]f32 // fill / tint, straight alpha
	color2 [4]f32 // border color, gradient end, or shadow color
	radii  [4]f32 // corner radii: top-left, top-right, bottom-right, bottom-left
	border [4]f32 // border widths: top, right, bottom, left
	uv     [4]f32 // atlas u0, v0, u1, v1
	params [4]f32 // quad: gradient angle, gradient on; shadow: blur; line: thickness, mode, rise; .w = clip radius
	kind   u32
	page   u32 // atlas page for mono/poly
	flags  u32
	xform  u32 // own transform (low 16 bits) and the clip's (high), indices into Scene.xforms
}

// ContentMask clips a primitive to a (rounded) rectangle.
pub struct ContentMask {
pub:
	bounds prim.Rect = prim.rect(-1.0e6, -1.0e6, 2.0e6, 2.0e6)
	radius f32
	xform  u32 // the space bounds are in (Scene.xforms index)
}

pub struct Quad {
pub:
	bounds         prim.Rect
	background     prim.Color
	gradient_to    ?prim.Color
	gradient_angle f32 // degrees, 0 = left to right
	border_widths  prim.Edges
	border_color   prim.Color
	radii          prim.Corners
}

pub struct Shadow {
pub:
	bounds prim.Rect // the box casting the shadow (already offset)
	radii  prim.Corners
	blur   f32
	color  prim.Color
}

pub struct Sprite {
pub:
	bounds prim.Rect
	tile   AtlasTile
	color  prim.Color = prim.white // tint for mono; opacity uses .a for poly
	radii  prim.Corners
	glyph  bool // text coverage (mono, or subpixel when poly): corrected and blended per channel
}

pub struct LinePrim {
pub:
	bounds    prim.Rect
	color     prim.Color
	thickness f32 = 1
	wavy      bool
	// A chart line crossing a 1 px column: the shader antialiases it exactly. thickness is
	// its vertical extent, rise how far it travels down the column; bounds are padded 1 px.
	column bool
	rise   f32
}

fn rect4(r prim.Rect) [4]f32 {
	return [r.x, r.y, r.width, r.height]!
}

fn corners4(c prim.Corners) [4]f32 {
	return [c.top_left, c.top_right, c.bottom_right, c.bottom_left]!
}

fn base(kind Kind, bounds prim.Rect, mask ContentMask) Instance {
	return Instance{
		bounds: rect4(bounds)
		clip:   rect4(mask.bounds)
		params: [f32(0), 0, 0, mask.radius]!
		kind:   u32(kind)
		xform:  mask.xform << 16
	}
}

pub fn (q Quad) instance(mask ContentMask) Instance {
	mut i := base(.quad, q.bounds, mask)
	i.color = q.background.to_array()
	i.radii = corners4(q.radii)
	b := q.border_widths
	i.border = [b.top, b.right, b.bottom, b.left]!
	if g := q.gradient_to {
		i.color2 = g.to_array()
		i.params[0] = q.gradient_angle
		i.params[1] = 1
		i.flags = 1
	}
	if b.top > 0 || b.right > 0 || b.bottom > 0 || b.left > 0 {
		i.color2 = q.border_color.to_array()
		i.flags |= 2
	}
	return i
}

pub fn (s Shadow) instance(mask ContentMask) Instance {
	// The quad grows by the blur extent so the falloff fits inside it.
	pad := s.blur * 1.5
	mut i := base(.shadow, prim.rect(s.bounds.x - pad, s.bounds.y - pad, s.bounds.width + pad * 2,

		s.bounds.height + pad * 2), mask)
	i.color2 = s.color.to_array()
	i.radii = corners4(s.radii)
	i.params[0] = s.blur
	i.border = [pad, pad, pad, pad]! // inset of the casting box inside bounds
	return i
}

pub fn (s Sprite) instance(kind Kind, mask ContentMask) Instance {
	mut i := base(kind, s.bounds, mask)
	i.color = s.color.to_array()
	i.uv = [s.tile.u0, s.tile.v0, s.tile.u1, s.tile.v1]!
	i.page = u32(s.tile.page)
	i.radii = corners4(s.radii)
	if s.glyph {
		i.flags = 1
	}
	return i
}

pub fn (l LinePrim) instance(mask ContentMask) Instance {
	mut i := base(.line, l.bounds, mask)
	i.color = l.color.to_array()
	i.params[0] = l.thickness
	i.params[1] = if l.column {
		f32(2)
	} else if l.wavy {
		f32(1)
	} else {
		f32(0)
	}
	i.params[2] = l.rise
	return i
}

// Instance bounds as a rect, for overlap tests.
pub fn (i &Instance) rect() prim.Rect {
	return prim.rect(i.bounds[0], i.bounds[1], i.bounds[2], i.bounds[3])
}

// screen_rect is where the instance lands on screen: its rotated bounding box.
pub fn (i &Instance) screen_rect(xforms []Rigid) prim.Rect {
	own := int(i.xform & 0xffff)
	if own == 0 || own >= xforms.len {
		return i.rect()
	}
	return xforms[own].bbox(i.rect())
}
