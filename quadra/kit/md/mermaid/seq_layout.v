// Sequence diagram layout: participants in columns far enough apart for the messages and
// notes between them, items stacked top to bottom, frames around what they contain.
module mermaid

import quadra.prim

struct SeqFrame {
mut:
	word     string
	text     string
	top      f32
	branches []f32
	labels   []string
	x0       f32 = 1e9
	x1       f32 = -1e9
	inner    int // nesting levels inside
}

fn (mut fr SeqFrame) cover(x0 f32, x1 f32) {
	fr.x0, fr.x1 = prim.min(fr.x0, x0), prim.max(fr.x1, x1)
}

// columns gives each participant's center, spread so every label fits.
fn columns(s &Seq, widths []f32, tbs []TextBox) []f32 {
	n := s.actors.len
	mut gap := []f32{len: n} // gap[i]: from center i to center i + 1
	for i in 0 .. n - 1 {
		gap[i] = (widths[i] + widths[i + 1]) / 2 + 36
	}
	mut left := f32(0) // room needed left of the first participant
	for k, it in s.items {
		w := tbs[k].width
		if it.kind == .message && it.from != it.to {
			a, b := if it.from < it.to { it.from, it.to } else { it.to, it.from }
			spread(mut gap, a, b, w + 40)
		} else if it.kind == .message || (it.kind == .note && it.side == 'right') {
			room := if it.kind == .message { w + 50 } else { w + 40 }
			if it.from < n - 1 {
				spread(mut gap, it.from, it.from + 1, room)
			}
		} else if it.kind == .note && it.side == 'left' {
			if it.from > 0 {
				spread(mut gap, it.from - 1, it.from, w + 40)
			} else {
				left = prim.max(left, w + 40)
			}
		} else if it.kind == .note && it.from != it.to {
			spread(mut gap, it.from, it.to, w - 16)
		}
	}
	mut c := []f32{len: n}
	c[0] = left
	for i in 1 .. n {
		c[i] = c[i - 1] + gap[i - 1]
	}
	return c
}

// spread widens the gaps between centers a and b evenly until they add up to need.
fn spread(mut gap []f32, a int, b int, need f32) {
	mut have := f32(0)
	for i in a .. b {
		have += gap[i]
	}
	if have < need {
		for i in a .. b {
			gap[i] += (need - have) / f32(b - a)
		}
	}
}

fn layout_seq(s &Seq, mut m Measure) Drawing {
	size := m.base.size
	small := size * 0.85
	mut d := Drawing{}
	if s.actors.len == 0 {
		return d
	}
	heads := s.actors.map(m.box(it.label, size, 500))
	widths := heads.map(prim.max(it.width + 24, 80))
	mut tbs := []TextBox{cap: s.items.len}
	for it in s.items {
		tbs << m.box(it.text, if it.kind == .message { size * 0.92 } else { size }, 400)
	}
	c := columns(s, widths, tbs)
	head_h := if s.actors.any(it.actor) { f32(58) } else { 38 }
	mut back := Drawing{} // frames and lifelines, under the rest
	mut y := head_h + 24
	mut stack := []SeqFrame{}
	mut num := 0
	for k, it in s.items {
		tb := tbs[k]
		mut x0, mut x1 := f32(0), f32(0)
		match it.kind {
			.message {
				a, b := c[it.from], c[it.to]
				label_h := prim.max(tb.height, m.line_h(size * 0.92))
				if it.from == it.to {
					d.text_ops(m, tb, a + 36 + tb.width / 2, y + label_h / 2, .text)
					y += label_h + 4
					pts := [prim.pt(a, y), prim.pt(a + 30, y),
						prim.pt(a + 30, y + 22), prim.pt(a, y + 22)]
					d.message(pts, it, num, s.autonumber, mut m)
					x0, x1 = a, a + 40 + tb.width
					y += 22 + 20
				} else {
					d.text_ops(m, tb, (a + b) / 2, y + label_h / 2, .text)
					y += label_h + 6
					d.message([prim.pt(a, y), prim.pt(b, y)], it, num, s.autonumber, mut m)
					x0, x1 = prim.min(a, b), prim.max(a, b)
					y += 22
				}
				num++
			}
			.note {
				w := prim.max(tb.width + 20, if it.from != it.to {
					c[it.to] - c[it.from] + 40
				} else {
					0
				})
				x0 = match it.side {
					'left' { c[it.from] - 12 - w }
					'right' { c[it.from] + 12 }
					else { (c[it.from] + c[it.to]) / 2 - w / 2 }
				}

				x1 = x0 + w
				h := tb.height + 14
				d.ops << Op{
					kind:   .box
					rect:   prim.rect(x0, y, w, h)
					radius: 3
					bg:     .note
					border: .line
				}
				d.text_ops(m, tb, x0 + w / 2, y + h / 2, .text)
				y += h + 16
			}
			.open {
				stack << SeqFrame{
					word: it.side
					text: it.text
					top:  y
				}
				y += m.line_h(small) + 18
			}
			.branch {
				if stack.len > 0 {
					stack[stack.len - 1].branches << y
					stack[stack.len - 1].labels << it.text
				}
				y += m.line_h(small) + 14
			}
			.close {
				if stack.len > 0 {
					mut fr := stack.pop()
					back.frame(mut fr, y, mut m)
					if stack.len > 0 {
						stack[stack.len - 1].cover(fr.x0, fr.x1)
						stack[stack.len - 1].inner = if fr.inner + 1 > stack[stack.len - 1].inner {
							fr.inner + 1
						} else {
							stack[stack.len - 1].inner
						}
					}
				}
				y += 18
			}
		}

		if x1 > x0 && stack.len > 0 {
			stack[stack.len - 1].cover(x0, x1)
		}
	}
	bottom := y + 4
	for i, a in s.actors {
		back.poly([prim.pt(c[i], head_h), prim.pt(c[i], bottom)], 1, true, .line)
		d.participant(a, heads[i], c[i], 0, widths[i], head_h, m)
		d.participant(a, heads[i], c[i], bottom, widths[i], head_h, m)
	}
	mut out := Drawing{
		ops: back.ops
	}
	out.ops << d.ops
	out.normalize(8)
	return out
}
