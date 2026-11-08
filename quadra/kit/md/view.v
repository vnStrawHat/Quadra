// Markdown view: md.view(src) renders parsed blocks with the kit theme (blocks.v). Text is
// rich text (links call on_link), code is highlighted by its language's tokenizer, math and
// Mermaid diagrams are drawn natively (embeds.v). track() reports where each block is, for
// outlines and scroll sync.
module md

import quadra.core
import quadra.prim
import quadra.kit
import quadra.layout
import quadra.ui

// Positions is where the blocks of the last layout are: their tops relative to the view's
// top, and their source lines. The app owns it and passes it to track().
@[heap]
pub struct Positions {
pub mut:
	tops   []f32
	lines  []int
	height f32
}

// top_of is the top of the block holding source line, for scrolling to it.
pub fn (p &Positions) top_of(line int) f32 {
	mut best := f32(0)
	for k, l in p.lines {
		if l > line || k >= p.tops.len {
			break
		}
		best = p.tops[k]
	}
	return best
}

// y_of is where source line (fractional: a scroll position in lines) is in the view,
// interpolated between the blocks around it, so scroll sync moves smoothly.
pub fn (p &Positions) y_of(line f32) f32 {
	n := if p.tops.len < p.lines.len { p.tops.len } else { p.lines.len }
	if n == 0 || line <= f32(p.lines[0]) {
		return 0
	}
	for k in 0 .. n - 1 {
		a, b := f32(p.lines[k]), f32(p.lines[k + 1])
		if line < b {
			return p.tops[k] + (line - a) / (b - a) * (p.tops[k + 1] - p.tops[k])
		}
	}
	return p.tops[n - 1]
}

// line_at is the source line of the block at y (relative to the view's top).
pub fn (p &Positions) line_at(y f32) int {
	mut best := 0
	for k, t in p.tops {
		if t > y || k >= p.lines.len {
			break
		}
		best = p.lines[k]
	}
	return best
}

@[heap]
pub struct MarkdownView {
	kit.Lazy
	src string
mut:
	link  ?core.AnyListener
	base  string // the document's folder: relative image paths start there
	pos   &Positions = unsafe { nil }
	memo  &Memo      = unsafe { nil }
	scale f32        = 1
	col   &ui.Div    = unsafe { nil }
}

// view parses src at layout time.
pub fn view(src string) &MarkdownView {
	return &MarkdownView{
		src: src
	}
}

// on_link gets the url of a clicked link.
pub fn (self_ &MarkdownView) on_link(l core.Listener[string]) &MarkdownView {
	mut v := unsafe { self_ }
	v.link = l.any
	return v
}

// base sets the folder relative image paths are read from.
pub fn (self_ &MarkdownView) base(dir string) &MarkdownView {
	mut v := unsafe { self_ }
	v.base = dir
	return v
}

// track writes the blocks' positions into p at every prepaint.
pub fn (self_ &MarkdownView) track(p &Positions) &MarkdownView {
	mut v := unsafe { self_ }
	v.pos = unsafe { p }
	return v
}

// memo keeps diagrams between builds (pass the same Memo every render).
pub fn (self_ &MarkdownView) memo(m &Memo) &MarkdownView {
	mut v := unsafe { self_ }
	v.memo = unsafe { m }
	return v
}

// scale makes the text larger or smaller (1 = the theme's reading size).
pub fn (self_ &MarkdownView) scale(f f32) &MarkdownView {
	mut v := unsafe { self_ }
	v.scale = f
	return v
}

pub fn (mut v MarkdownView) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut v, mut cx)
	return v.layout_inner(el, mut cx)
}

pub fn (mut v MarkdownView) build_tree(mut cx core.LayoutCx) core.Element {
	t := kit.theme_of(cx.app)
	blocks := parse(v.src)
	mut col :=
		ui.div().flex_col().w_full().text_color(t.p.text).text_size(v.body_size(t)).line_height(1.5)
	mut lines := []int{cap: blocks.len}
	if v.memo != unsafe { nil } {
		v.memo.start()
	}
	// Blocks are 16 apart, the items of one list 4 (specs/15); each block stays one child
	// so the reading position maps to blocks.
	mut prev := Kind.front
	for b in blocks {
		if b.kind == .front && blocks.len > 1 {
			continue // front matter is metadata
		}
		listed := b.kind in [.bullet, .ordered]
		gap := if lines.len == 0 {
			f32(0)
		} else if listed && prev in [.bullet, .ordered] {
			4
		} else {
			16
		}
		col.child(ui.div().flex_col().pt(gap).child(v.block(b, t)))
		lines << b.line
		prev = b.kind
	}
	if v.pos != unsafe { nil } {
		v.pos.lines = lines
	}
	if v.memo != unsafe { nil } {
		v.memo.finish()
	}
	v.col = col
	return col
}

pub fn (mut v MarkdownView) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	if v.pos != unsafe { nil } && v.col != unsafe { nil } {
		kids := cx.tree.children(layout.NodeId(v.col.layout_node()))
		v.pos.tops = kids.map(cx.tree.rect(it).y)
		v.pos.height = bounds.height
	}
	v.Lazy.prepaint(bounds, mut cx)
}

// body_size is the reading size (specs/15): 16 px, times the view's zoom.
fn (v &MarkdownView) body_size(t &kit.Theme) f32 {
	return t.scaled(16) * v.scale
}

fn (mut v MarkdownView) spans(s string, t &kit.Theme) []ui.Span {
	mut out := []ui.Span{}
	for sp in parse_inline(s) {
		if sp.math {
			out << v.inline_math(sp.text)
			continue
		}
		mut u := ui.span(if sp.image != '' { '[${sp.text}]' } else { sp.text })
		if sp.bold {
			u = u.bold()
		}
		if sp.italic {
			u = u.italic()
		}
		if sp.strike {
			u = u.strike().color(t.p.text_3)
		}
		if sp.code {
			u = u.family(t.mono_family).color(t.code.syntax.str).size(v.body_size(t) * 0.875)
		}
		if sp.link != '' {
			u = u.link(sp.link).color(t.p.accent_text)
		}
		out << u
	}
	return out
}

fn (v &MarkdownView) rich(spans []ui.Span) &ui.RichEl {
	r := ui.rich_text(spans)
	if l := v.link {
		return r.on_link(core.Listener[string]{ any: l })
	}
	return r
}
