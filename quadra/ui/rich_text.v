// Rich text element: spans of mixed style and links in one wrapped paragraph (specs/08).
module ui

import quadra.core
import quadra.prim
import quadra.gpu
import quadra.layout
import quadra.platform
import quadra.text as txt

// Span is a value builder: ui.span('docs').bold().link('https://...').
pub struct Span {
pub mut:
	text   string
	patch  TextPatch
	link   string     // non-empty: underlined and clickable (RichEl.on_link)
	box    ?InlineBox // an inline object in place of the text (inline math)
	strike bool       // a line through the text
}

pub fn (s Span) strike() Span {
	mut c := s
	c.strike = true
	return c
}

// inline_box is a span holding an object that flows with the text like a word.
pub fn inline_box(b InlineBox) Span {
	return Span{
		box: b
	}
}

pub fn span(s string) Span {
	return Span{
		text: s
	}
}

pub fn (s Span) bold() Span {
	mut c := s
	c.patch.set_weight(700)
	return c
}

pub fn (s Span) weight(w int) Span {
	mut c := s
	c.patch.set_weight(w)
	return c
}

pub fn (s Span) italic() Span {
	mut c := s
	c.patch.set_italic(true)
	return c
}

pub fn (s Span) color(v prim.Color) Span {
	mut c := s
	c.patch.set_color(v)
	return c
}

pub fn (s Span) size(v f32) Span {
	mut c := s
	c.patch.set_size(v)
	return c
}

pub fn (s Span) family(f string) Span {
	mut c := s
	c.patch.set_family(f)
	return c
}

pub fn (s Span) link(url string) Span {
	mut c := s
	c.link = url
	return c
}

struct LinkHit {
mut:
	el  &RichEl = unsafe { nil }
	url string
}

// ponytail: not selectable yet (TextEl is); add when rich text shows copyable documents.
@[heap]
pub struct RichEl {
pub mut:
	spans []Span
	patch TextPatch // applies to every span, under the span's own patch
mut:
	base   txt.TextStyle
	runs   []txt.Run
	ts     &txt.TextSystem = unsafe { nil }
	laid   &txt.RichText   = unsafe { nil }
	wrap_w f32
	node   layout.NodeId
	link   ?core.AnyListener
	hits   []LinkHit
}

pub fn rich_text(spans []Span) &RichEl {
	return &RichEl{
		spans: spans
	}
}

pub fn (self_ &RichEl) size(v f32) &RichEl {
	mut r := unsafe { self_ }
	r.patch.set_size(v)
	return r
}

pub fn (self_ &RichEl) color(c prim.Color) &RichEl {
	mut r := unsafe { self_ }
	r.patch.set_color(c)
	return r
}

// on_link gets the url of a clicked link.
pub fn (self_ &RichEl) on_link(l core.Listener[string]) &RichEl {
	mut r := unsafe { self_ }
	r.link = l.any
	return r
}

fn (r &RichEl) fits(max_w f32) bool {
	return r.laid != unsafe { nil } && r.laid.width <= max_w
		&& (max_w <= r.wrap_w || !r.laid.width_breaks)
}

fn measure_rich(ctx voidptr, kw f32, _ f32, aw f32, _ f32) prim.Size {
	mut r := unsafe { &RichEl(ctx) }
	limit := if kw != layout.unknown { kw } else { aw }
	max_w := if limit >= layout.unbounded { f32(1.0e9) } else { limit }
	if !r.fits(max_w) {
		r.laid, r.wrap_w = r.ts.layout_rich(r.runs, max_w), max_w
	}
	return prim.size(if kw != layout.unknown { kw } else { r.laid.width }, r.laid.height)
}

pub fn (mut r RichEl) request_layout(mut cx core.LayoutCx) layout.NodeId {
	base := r.patch.apply(cx.text_style)
	if base != r.base || r.runs.len != r.spans.len {
		r.base = base
		r.laid = unsafe { nil }
		r.runs = []txt.Run{cap: r.spans.len}
		for sp in r.spans {
			st := sp.patch.apply(base)
			if b := sp.box {
				w, asc, desc := b.box_size(mut cx.text, st)
				r.runs << txt.Run{
					text:     ''
					style:    st
					box_w:    prim.max(w, 0.01)
					box_asc:  asc
					box_desc: desc
				}
			} else {
				r.runs << txt.Run{
					text:  sp.text
					style: st
				}
			}
		}
	}
	r.ts = cx.text
	r.node = cx.tree.leaf(text_leaf_style, measure_rich, voidptr(r))
	return r.node
}

fn (mut r RichEl) layout_at(w f32) &txt.RichText {
	if !r.fits(w + 0.5) {
		r.laid, r.wrap_w = r.ts.layout_rich(r.runs, w + 0.5), w + 0.5
	}
	return r.laid
}

pub fn (mut r RichEl) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.register_text(r.spans.map(it.text).join(''), bounds)
	if r.link == none {
		return
	}
	rt := r.layout_at(bounds.width)
	r.hits = []LinkHit{}
	mut rects := []prim.Rect{}
	for line in rt.lines {
		for p in line.pieces {
			if r.spans[p.run].link != '' {
				r.hits << LinkHit{r, r.spans[p.run].link}
				rects << prim.rect(bounds.x + p.x, bounds.y + line.y, p.width, line.height)
			}
		}
	}
	for k, rect in rects { // the hits array is complete: pointers into it stay valid
		cx.enter('link${k}')
		hb := cx.insert_hitbox(rect, core.HitboxOptions{
			cursor: platform.Cursor.pointer
		})
		cx.on_mouse(hb, .click, core.AnyListener{
			holder: voidptr(&r.hits[k])
			call:   rich_link_click
		}, false)
		cx.leave()
	}
}

fn rich_link_click(holder voidptr, _ voidptr, _ voidptr, mut cx core.Cx) {
	h := unsafe { &LinkHit(holder) }
	if l := h.el.link {
		url := h.url
		cx.app.invoke(l, &url, cx.win, true)
	}
}

pub fn (mut r RichEl) paint(bounds prim.Rect, mut cx core.PaintCx) {
	rt := r.layout_at(bounds.width)
	clip := cx.clip_rect()
	for line in rt.lines {
		top := bounds.y + line.y
		if top + line.height < clip.y - 2 || top > clip.y + clip.height + 2 {
			continue
		}
		for p in line.pieces {
			st := r.runs[p.run].style
			x := bounds.x + p.x
			if mut b := r.spans[p.run].box {
				b.paint_box(prim.pt(x, top + line.baseline), st, mut cx)
				continue
			}
			cx.glyphs(p.layout, x, top + line.baseline, st.color)
			if r.spans[p.run].strike {
				asc, _ := cx.text.line_metrics(st)
				_, thick := cx.text.underline(st)
				cx.quad(gpu.Quad{
					bounds:     prim.rect(x, top + line.baseline - asc * 0.3, p.width, prim.max(1,
						thick))
					background: st.color
				})
			}
			if r.spans[p.run].link != '' {
				pos, thick := cx.text.underline(st)
				cx.quad(gpu.Quad{
					bounds:     prim.rect(x, top + line.baseline + pos, p.width, prim.max(1, thick))
					background: st.color
				})
			}
		}
	}
}
