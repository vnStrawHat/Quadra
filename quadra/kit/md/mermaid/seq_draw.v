// Sequence diagram drawing: participant boxes and figures, messages with their heads and
// numbers, and frames with their keyword tab, condition and branches.
module mermaid

import quadra.prim

fn (mut d Drawing) message(pts []prim.Point, it SeqItem, num int, numbered bool, mut m Measure) {
	line := cut_end(pts, head_len(it.head, 1.5))
	d.poly(line, 1.5, it.dashed, .accent)
	// An open head is drawn from the tip, so the line is not shortened: aim it along the line.
	from := if dist(line.last(), pts.last()) < 0.5 { pts[pts.len - 2] } else { line.last() }
	d.head(pts.last(), from, it.head, .accent)
	if numbered {
		p := pts[0]
		d.ops << Op{
			kind:   .box
			rect:   prim.rect(p.x - 9, p.y - 9, 18, 18)
			radius: 9
			bg:     .accent
		}
		tb := m.box('${num + 1}', m.base.size * 0.75, 600)
		d.text_ops(m, tb, p.x, p.y, .on_accent)
	}
}

// participant draws a participant at the top (or bottom) of its lifeline at x.
fn (mut d Drawing) participant(a Actor, tb TextBox, x f32, top f32, w f32, h f32, m &Measure) {
	if !a.actor {
		d.ops << Op{
			kind:   .box
			rect:   prim.rect(x - w / 2, top, w, h)
			radius: 4
			bg:     .panel
			border: .line
		}
		d.text_ops(m, tb, x, top + h / 2, .text)
		return
	}
	d.ops << Op{
		kind:   .box
		rect:   prim.rect(x - 7, top + 2, 14, 14)
		radius: 7
		border: .text
	}
	d.poly([prim.pt(x, top + 16), prim.pt(x, top + 30)], 1.5, false, .text)
	d.poly([prim.pt(x - 10, top + 21), prim.pt(x + 10, top + 21)], 1.5, false, .text)
	d.poly([prim.pt(x - 8, top + 40), prim.pt(x, top + 30), prim.pt(x + 8, top + 40)], 1.5, false,
		.text)
	d.text_ops(m, tb, x, top + h - tb.height / 2, .text)
}

// frame draws a closed frame that ends at y, padded by its nesting.
fn (mut d Drawing) frame(mut fr SeqFrame, y f32, mut m Measure) {
	small := m.base.size * 0.85
	pad := 12 + 10 * f32(fr.inner)
	if fr.x1 < fr.x0 {
		fr.x0, fr.x1 = 0, 120
	}
	fr.x0, fr.x1 = fr.x0 - pad, fr.x1 + pad
	w := fr.x1 - fr.x0
	d.ops << Op{
		kind:   .box
		rect:   prim.rect(fr.x0, fr.top, w, y + 6 - fr.top)
		radius: 3
		bg:     .frame
		border: .line
	}
	tab := m.box(fr.word, small, 600)
	tab_h := m.line_h(small) + 6
	d.ops << Op{
		kind:   .box
		rect:   prim.rect(fr.x0, fr.top, tab.width + 16, tab_h)
		radius: 3
		bg:     .panel
		border: .line
	}
	d.text_ops(m, tab, fr.x0 + 8 + tab.width / 2, fr.top + tab_h / 2, .text)
	if fr.text != '' {
		d.text_left(m, '[${fr.text}]', fr.x0 + tab.width + 24, fr.top + tab_h / 2, small, 400,
			.text_2)
	}
	for i, by in fr.branches {
		d.poly([prim.pt(fr.x0, by), prim.pt(fr.x1, by)], 1, true, .line)
		if fr.labels[i] != '' {
			d.text_left(m, '[${fr.labels[i]}]', fr.x0 + 10, by + m.line_h(small) / 2 + 5, small,
				400, .text_2)
		}
	}
}
