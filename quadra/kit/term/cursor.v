// What the grid paints over the cells: the cursor, IME text, search matches, the hovered link.
module term

import quadra.core
import quadra.platform
import quadra.prim
import quadra.gpu
import quadra.kit
import quadra.text

fn (g &Grid) paint_cursor(bounds prim.Rect, st text.TextStyle, base f32, pal &kit.TerminalColors, mut cx core.PaintCx) {
	t := g.t
	s := t.screen
	row := s.y + t.offset
	if !s.cursor_on || row >= s.rows || t.exited != none {
		return
	}
	focused := cx.is_focused(t.focus.id)
	cell := s.lines[s.y].cells[clamp(s.x, 0, s.cols - 1)]
	w := if cell.flags & f_wide != 0 { t.cell.width * 2 } else { t.cell.width }
	r := prim.rect(bounds.x + f32(s.x) * t.cell.width, bounds.y + f32(row) * t.cell.height, w,
		t.cell.height)
	if focused {
		mut win := cx.win
		win.pw.set_ime_rect(prim.rect(r.x, r.y, 1, r.height))
	}
	if t.preedit != '' {
		l := cx.text.layout_line(t.preedit, st)
		cx.quad(gpu.Quad{ bounds: prim.rect(r.x, r.y, l.width, r.height), background: pal.background })
		cx.glyphs(l, r.x, r.y + base, pal.foreground)
		cx.quad(gpu.Quad{
			bounds:     prim.rect(r.x, r.y + base + 2, l.width, 1)
			background: pal.foreground
		})
		return
	}
	color := if t.b.cursor_color.a > 0 { t.b.cursor_color } else { pal.cursor }
	blinking := if t.screen.cursor_blink != 0 && t.b.cursor == .program {
		t.screen.cursor_blink == 1
	} else {
		t.b.cursor_blink
	}
	if focused && blinking {
		phase := int(cx.now_ms() / blink_ms)
		cx.request_paint_after(int(f64(phase + 1) * blink_ms - cx.now_ms()) + 1)
		if phase % 2 == 1 {
			return
		}
	}
	if !focused {
		cx.quad(gpu.Quad{
			bounds:        r
			border_widths: prim.edges(1)
			border_color:  color
		})
		return
	}
	match t.shape_now() {
		.bar {
			bar := if t.cell.width * 0.2 < 1 { f32(1) } else { t.cell.width * 0.2 }
			cx.quad(gpu.Quad{ bounds: prim.rect(r.x, r.y, bar, r.height), background: color })
		}
		.underline {
			cx.quad(gpu.Quad{
				bounds:     prim.rect(r.x, r.y + r.height - 2, r.width, 2)
				background: color
			})
		}
		.block {
			cx.quad(gpu.Quad{ bounds: r, background: color })
			if cell.ch > 32 {
				l := cx.text.layout_line(rune(cell.ch).str(), st)
				cx.glyphs(l, r.x, r.y + base, pal.background)
			}
		}
	}
}

// paint_hits draws the matches on a row from hit `start` and returns the first hit left.
fn (g &Grid) paint_hits(line int, y f32, start int, ed kit.EditorColors, mut cx core.PaintCx) int {
	t := g.t
	mut next := start
	for next < t.find.hits.len && t.find.hits[next].line <= line {
		h := t.find.hits[next]
		if h.line == line {
			cx.quad(gpu.Quad{
				bounds:     prim.rect(t.bounds.x + f32(h.col) * t.cell.width, y,
					f32(h.len) * t.cell.width, t.cell.height)
				background: if next == t.find.current { ed.find_current } else { ed.find_match }
			})
		}
		next++
	}
	return next
}

fn (g &Grid) paint_link(line int, baseline f32, pal &kit.TerminalColors, mut cx core.PaintCx) {
	t := g.t
	c0 := if line == t.hover.line { t.hover.c0 } else { 0 }
	c1 := if line == t.hover.end_line { t.hover.c1 } else { t.screen.cols }
	cx.quad(gpu.Quad{
		bounds:     prim.rect(t.bounds.x + f32(c0) * t.cell.width, baseline + 2,
			f32(c1 - c0) * t.cell.width, 1)
		background: pal.foreground
	})
}

// link_hitboxes give the hovered link, one per row it covers, the pointer while ctrl is held
// and the hint "Ctrl+click to open", as in the editor (specs/22, Links). No hint while a
// program reads the mouse: a ctrl-click is the program's then.
fn (mut g Grid) link_hitboxes(mut cx core.PrepaintCx) {
	g.hbs.clear()
	t := g.t
	if t.hover.url == '' || t.cell.height <= 0 {
		return
	}
	hint := if t.screen.mouse == .off { core.hint_ctrl_open } else { '' }
	top := t.top_line()
	cx.enter('links') // keyed: the rows' slots never shift the grid's siblings
	for line in t.hover.line .. t.hover.end_line + 1 {
		r := line - top
		if r < 0 || r >= t.screen.rows {
			continue
		}
		c0 := if line == t.hover.line { t.hover.c0 } else { 0 }
		c1 := if line == t.hover.end_line { t.hover.c1 } else { t.screen.cols }
		cx.enter('')
		g.hbs << cx.insert_hitbox(prim.rect(t.bounds.x + f32(c0) * t.cell.width, t.bounds.y +
			f32(r) * t.cell.height, f32(c1 - c0) * t.cell.width, t.cell.height), core.HitboxOptions{
			cursor:         platform.Cursor.pointer
			cursor_mods:    platform.Mods.ctrl
			hint:           hint
			hover_prepaint: true
		})
		cx.leave()
	}
	cx.leave()
}

// link_lit: the hovered link is underlined while ctrl is held over it.
fn (g &Grid) link_lit(mut cx core.PaintCx) bool {
	return g.hbs.len > 0 && cx.win.mods.has(.ctrl) && g.hbs.any(cx.is_hovered(it))
}
