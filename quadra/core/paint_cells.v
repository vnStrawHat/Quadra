// Painting monospace runs on a cell grid and the shared glyph sprite path.
module core

import math
import quadra.gpu
import quadra.prim
import quadra.text

// glyphs_cells paints a monospace run on a grid of `advance` px cells: each glyph sits at
// its column, its own offset inside the cluster kept. Whole device pixel cell widths and a
// fixed cell_width then hold, whatever the font's natural advance is. ASCII clusters only:
// the byte offset is the column.
pub fn (mut cx PaintCx) glyphs_cells(l &text.LineLayout, x f32, baseline f32, color prim.Color, advance f32) {
	if l.glyphs.len == 0 {
		return
	}
	k, t := cx.xf.k, cx.xf.t
	s := cx.scale * k
	dev_size := if k == 1 { l.size * s } else { f32(math.round(l.size * s * 4)) / 4 }
	base_y := f32(math.round((baseline * k + t.y) * cx.scale))
	tx := t.x * cx.scale
	c := cx.fade(color)
	mut cluster, mut base := -1, f32(0)
	for g in l.glyphs {
		if g.cluster != cluster {
			cluster, base = g.cluster, g.x
		}
		cell_x := x + f32(cluster) * advance + g.x - base
		gx := cell_x * s + tx
		fx := f32(math.floor(gx))
		phase := int((gx - fx) * text.subpixel_steps)
		tile := cx.text.glyph_tile(g.face, g.id, dev_size, phase)
		if tile.empty {
			continue
		}
		cx.glyph_sprite(tile, prim.rect(fx + tile.left,
			base_y - f32(math.round(g.y * s)) + tile.top, tile.tile.w, tile.tile.h), c)
	}
}

fn (mut cx PaintCx) glyph_sprite(tile text.GlyphTile, bounds prim.Rect, c prim.Color) {
	sprite := gpu.Sprite{
		bounds: bounds
		tile:   tile.tile
		color:  c
		glyph:  !tile.color
	}
	if tile.color || tile.subpixel {
		cx.scene.poly(sprite, cx.mask())
	} else {
		cx.scene.mono(sprite, cx.mask())
	}
}
