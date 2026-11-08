// Canvas: custom painting with rectangles, lines, paths and text. Rectangles and strokes
// (lines, stroked paths) go to the GPU directly, so animated strokes cost no atlas space;
// filled paths are rasterized on the CPU and cached by geometry and size.
module ui

import math
import quadra.core
import quadra.prim
import quadra.gpu
import quadra.layout
import quadra.text as txt
import quadra.vector

enum OpKind {
	rect
	fill_path
	stroke_path
	text
}

struct PaintOp {
	kind   OpKind
	rect   prim.Rect
	color  prim.Color
	radius f32
	width  f32
	path   vector.Path
	text   string
	style  txt.TextStyle
}

pub struct Painter {
mut:
	ops []PaintOp
}

pub fn (mut p Painter) fill_rect(r prim.Rect, color prim.Color, radius f32) {
	p.ops << PaintOp{
		kind:   .rect
		rect:   r
		color:  color
		radius: radius
	}
}

pub fn (mut p Painter) fill_path(path vector.Path, color prim.Color) {
	p.ops << PaintOp{
		kind:  .fill_path
		path:  path
		color: color
	}
}

pub fn (mut p Painter) stroke_path(path vector.Path, width f32, color prim.Color) {
	p.ops << PaintOp{
		kind:  .stroke_path
		path:  path
		width: width
		color: color
	}
}

pub fn (mut p Painter) line(x0 f32, y0 f32, x1 f32, y1 f32, width f32, color prim.Color) {
	mut path := vector.new_path()
	path.move_to(x0, y0)
	path.line_to(x1, y1)
	p.stroke_path(path, width, color)
}

// text draws a single line with its baseline at (x, y).
pub fn (mut p Painter) text(s string, x f32, y f32, style txt.TextStyle) {
	p.ops << PaintOp{
		kind:  .text
		rect:  prim.rect(x, y, 0, 0)
		text:  s
		style: style
	}
}

pub type PaintFn = fn (mut p Painter, size prim.Size)

@[heap]
pub struct CanvasEl {
pub mut:
	style layout.Style
mut:
	draw PaintFn = unsafe { nil }
	node layout.NodeId
}

// canvas fills its parent by default; size it with .w()/.h() through the style field.
pub fn canvas(draw PaintFn) &CanvasEl {
	return &CanvasEl{
		draw:  draw
		style: layout.Style{
			grow:   1
			width:  layout.pct(100)
			height: layout.pct(100)
		}
	}
}

pub fn (self_ &CanvasEl) size(w f32, h f32) &CanvasEl {
	mut c := unsafe { self_ }
	c.style.width, c.style.height = layout.px(w), layout.px(h)
	c.style.grow = 0
	return c
}

pub fn (mut c CanvasEl) request_layout(mut cx core.LayoutCx) layout.NodeId {
	c.node = cx.tree.node(c.style, [])
	return c.node
}

pub fn (mut c CanvasEl) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {}

pub fn (mut c CanvasEl) paint(bounds prim.Rect, mut cx core.PaintCx) {
	mut p := Painter{}
	c.draw(mut p, bounds.size())
	cx.push_clip(bounds, 0)
	for op in p.ops {
		match op.kind {
			.rect {
				cx.quad(gpu.Quad{
					bounds:     op.rect.translate(bounds.x, bounds.y)
					background: op.color
					radii:      prim.corners(op.radius)
				})
			}
			.fill_path {
				paint_fill(mut cx, op, bounds)
			}
			.stroke_path {
				for pl in op.path.flatten(1, bounds.x, bounds.y, 0.25 / cx.scale) {
					cx.stroke(pl.points, op.width, op.color, pl.closed)
				}
			}
			.text {
				l := cx.text.layout_line(op.text, op.style)
				cx.glyphs(l, bounds.x + op.rect.x, bounds.y + op.rect.y, op.style.color)
			}
		}
	}
	cx.pop_clip()
}

fn paint_fill(mut cx core.PaintCx, op PaintOp, bounds prim.Rect) {
	s := cx.scale
	// Flatten once at device scale to find the pixel box, then rasterize relative to it.
	lines := op.path.flatten(s, bounds.x * s, bounds.y * s, 0.25)
	pad := f32(1)
	mut x0, mut y0 := f32(1e9), f32(1e9)
	mut x1, mut y1 := f32(-1e9), f32(-1e9)
	for pl in lines {
		for q in pl.points {
			x0, y0 = prim.min(x0, q.x), prim.min(y0, q.y)
			x1, y1 = prim.max(x1, q.x), prim.max(y1, q.y)
		}
	}
	if x1 < x0 {
		return
	}
	bx, by := f32(math.floor(x0 - pad)), f32(math.floor(y0 - pad))
	w, h := int(math.ceil(x1 + pad - bx)), int(math.ceil(y1 + pad - by))
	if w <= 0 || h <= 0 || w > 4096 || h > 4096 {
		return
	}
	fx, fy := bx - bounds.x * s, by - bounds.y * s
	key := 'path:${op.path.hash()}:${s}:${w}x${h}:${fx}:${fy}'
	mut app := cx.app
	entry := app.cached_image(key) or {
		mut cov := vector.new_coverage(w, h)
		// Canvas-local geometry at device scale, moved into the bitmap's pixel box.
		pts := op.path.flatten(s, -fx, -fy, 0.25)
		cov.fill(pts)
		app.image_put(key, core.Bitmap{w, h, cov.to_u8(), true})
	}
	cx.sprite(prim.rect(bx / s, by / s, f32(w) / s, f32(h) / s), entry, op.color)
}
