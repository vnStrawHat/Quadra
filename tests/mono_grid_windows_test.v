// The monospace grid with real fonts (specs/04, layout_mono): CJK falls back to a font whose
// glyphs are not two Consolas cells wide, yet every column after it stays on the grid.
module main

import math
import quadra.text
import quadra.text.dwrite

fn test_fallback_glyphs_keep_the_mono_grid() {
	mut ts := text.new_system(dwrite.new_backend()!)
	st := text.TextStyle{
		family: 'Consolas'
		size:   14
	}
	cell := ts.layout_line('M', st).width
	for s in ["| 3 | F-123 | Việt | Med | 漢字 | Won't fix |", '| 😀 | tiếng |', '漢|字|ab'] {
		l := ts.layout_mono(s, st, cell)
		for i in 0 .. s.len {
			if s[i] == `|` {
				want := f32(text.display_width(s[..i])) * cell
				assert math.abs(l.x_for_index(i) - want) < 0.01, '${s} at ${i}'
			}
		}
		// Glyphs stay in cluster order (PaintCx.glyphs_range searches them by cluster).
		for k in 1 .. l.glyphs.len {
			assert l.glyphs[k].cluster >= l.glyphs[k - 1].cluster
		}
	}
}
