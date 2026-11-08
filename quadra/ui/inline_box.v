// InlineBox: an object that sits inside rich text like a word (inline math). Rich text asks
// for its size when it lays out a line and has it paint itself at the place it got.
module ui

import quadra.core
import quadra.prim
import quadra.text as txt

pub interface InlineBox {
	// box_size is the width, the height above the baseline and the depth below it, for
	// text of style st (the span's style: its size and color).
	box_size(mut ts txt.TextSystem, st txt.TextStyle) (f32, f32, f32)
	// paint_box paints with the baseline's left end at origin.
mut:
	paint_box(origin prim.Point, st txt.TextStyle, mut cx core.PaintCx)
}
