// Decorations painted over the text: underlines, overline, strike-through.
module term

import math
import quadra.core
import quadra.gpu
import quadra.kit
import quadra.prim

fn (g &Grid) hline(x f32, y f32, w f32, color prim.Color, mut cx core.PaintCx) {
	cx.quad(gpu.Quad{
		bounds:     prim.rect(x, y, w, 1)
		background: color
	})
}

// decorate draws the underline (any style, in its SGR 58 color), overline and strike-through
// of a run of n cells.
fn (g &Grid) decorate(c Cell, x int, n int, y f32, baseline f32, pal &kit.TerminalColors, mut cx core.PaintCx) {
	if c.flags & (f_underline | f_strike) == 0 && c.pad == 0 {
		return
	}
	t := g.t
	e := t.screen.extra_of(c.pad)
	if c.flags & (f_underline | f_strike) == 0 && !e.over {
		return
	}
	fg, _ := t.screen.colors(c, pal)
	x0, w := t.bounds.x + f32(x) * t.cell.width, f32(n) * t.cell.width
	if c.flags & f_underline != 0 {
		ul := if e.ul != 0 { t.screen.color_of(e.ul, fg, pal) } else { fg }
		g.underline(c.flags, x0, baseline + 2, w, ul, mut cx)
	}
	if e.over {
		g.hline(x0, y, w, fg, mut cx)
	}
	if c.flags & f_strike != 0 {
		g.hline(x0, y + t.cell.height * 0.5, w, fg, mut cx)
	}
}

fn (g &Grid) underline(flags u16, x0 f32, y f32, w f32, color prim.Color, mut cx core.PaintCx) {
	if flags & f_double != 0 {
		g.hline(x0, y, w, color, mut cx)
		g.hline(x0, y + 2, w, color, mut cx)
	} else if flags & f_curly != 0 {
		// A sine wave, one period every four pixels, stroked as one polyline.
		steps := int(w / 2) + 1
		mut points := []prim.Point{cap: steps + 1}
		for i in 0 .. steps + 1 {
			px := f32(i) * 2
			points << prim.Point{x0 + (if px < w { px } else { w }), y + 1 +
				f32(math.sin(f64(px) * math.pi / 2)) * 1.5}
		}
		cx.stroke(points, 1, color, false)
	} else if flags & f_dotted != 0 {
		mut px := f32(0)
		for px < w {
			cx.quad(gpu.Quad{
				bounds:     prim.rect(x0 + px, y, 1, 1)
				background: color
			})
			px += 2
		}
	} else if flags & f_dashed != 0 {
		dash := f32(math.max(2.0, g.t.cell.width * 0.5))
		mut px := f32(0)
		for px < w {
			end := if px + dash < w { dash } else { w - px }
			cx.quad(gpu.Quad{
				bounds:     prim.rect(x0 + px, y, end, 1)
				background: color
			})
			px += dash * 2
		}
	} else {
		g.hline(x0, y, w, color, mut cx)
	}
}
