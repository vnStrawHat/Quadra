// Rich text (text/rich.v, ui/rich_text.v): wrapping across runs, glued words, newlines,
// one baseline per line, links.
module main

import quadra.core
import quadra.testing
import quadra.text
import quadra.text.fake
import quadra.ui

fn style(size f32, weight int) text.TextStyle {
	return text.TextStyle{
		size:   size
		weight: weight
	}
}

// The fake backend advances 0.6 em per grapheme: 6 px at size 10.
fn test_breaks_at_spaces_and_keeps_glued_words() {
	mut ts := text.new_system(fake.new())
	runs := [run('hello wor', style(10, 400)), run('ld big', style(10, 700))]
	rt := ts.layout_rich(runs, 60) // 'hello world' is 66 px: 'world' moves down whole
	assert rt.lines.len == 2
	assert rt.width_breaks
	l0, l1 := rt.lines[0], rt.lines[1]
	assert l0.pieces.len == 1
	assert l0.pieces[0].end == 6 // 'hello ' hangs its space
	assert l0.width == 30 // without the trailing space
	assert l1.pieces.len == 2
	assert l1.pieces[0].run == 0 && l1.pieces[0].start == 6
	assert l1.pieces[1].x == 18 // after 'wor'
	assert l1.y == l0.height
	wide := ts.layout_rich(runs, 1000)
	assert wide.lines.len == 1
	assert !wide.width_breaks
	assert wide.width == 90
}

fn test_newlines_end_lines() {
	mut ts := text.new_system(fake.new())
	rt := ts.layout_rich([run('a\n\nb', style(10, 400))], 1000)
	assert rt.lines.len == 3
	assert rt.lines[1].pieces.len == 0
	assert rt.lines[1].height == rt.lines[0].height
	assert rt.lines[2].pieces[0].start == 3
}

fn test_mixed_sizes_share_a_baseline() {
	mut ts := text.new_system(fake.new())
	small := ts.layout_rich([run('ab', style(10, 400))], 1000)
	big := ts.layout_rich([run('ab', style(20, 400))], 1000)
	mixed := ts.layout_rich([run('ab ', style(10, 400)), run('cd', style(20, 400))], 1000)
	assert mixed.lines.len == 1
	assert mixed.lines[0].baseline == big.lines[0].baseline
	assert mixed.lines[0].height == big.lines[0].height
	assert big.lines[0].baseline > small.lines[0].baseline
}

struct Doc {
mut:
	opened []string
}

@[markused]
pub fn (mut d Doc) render(mut cx core.ViewCx[Doc]) core.Element {
	return ui.div().test_id('doc').w(300).child(ui.rich_text([
		ui.span('docs').link('https://docs'),
		ui.span(' and '),
		ui.span('more').bold(),
	]).on_link(cx.listener(fn (mut d Doc, url &string, mut _ core.ViewCx[Doc]) {
		d.opened << *url
	})))
}

fn test_clicking_a_link_reports_its_url() {
	mut h := testing.new_harness(width: 400, height: 200)!
	v := h.open(Doc{}, width: 400, height: 200)!
	assert h.has_text('docs and more')
	r := h.find('doc') or { panic('no doc') }
	h.click_at(r.x + 60, r.y + 4) // ' and ': not a link
	h.click_at(r.x + 4, r.y + 4)
	mut cx := core.Cx{
		app: h.app
	}
	assert cx.peek(v.model()).opened == ['https://docs']
}

struct Lines {}

@[markused]
pub fn (mut l Lines) render(mut cx core.ViewCx[Lines]) core.Element {
	return ui.div().flex_col().child(ui.div().test_id('long').w(60).child(ui.text('many words that would wrap').truncate())).child(ui.div().test_id('short').w(60).child(ui.text('x').truncate()))
}

fn test_truncated_text_stays_on_one_line() {
	mut h := testing.new_harness(width: 400, height: 200)!
	h.open(Lines{}, width: 400, height: 200)!
	long := h.find('long') or { panic('no long') }
	short := h.find('short') or { panic('no short') }
	assert long.height == short.height
	assert long.width == 60
}

fn run(s string, st text.TextStyle) text.Run {
	return text.Run{
		text:  s
		style: st
	}
}

fn test_inline_boxes_flow_like_words() {
	mut ts := text.new_system(fake.new())
	runs := [run('ab ', style(10, 400)), text.Run{
		style:    style(10, 400)
		box_w:    30
		box_asc:  40
		box_desc: 12
	}, run(' cd', style(10, 400))]
	rt := ts.layout_rich(runs, 1000)
	assert rt.lines.len == 1
	l := rt.lines[0]
	assert l.pieces[1].width == 30 && l.pieces[1].x == l.pieces[0].width
	assert l.pieces[2].x == l.pieces[1].x + 30
	assert l.baseline == 40 && l.height == 52 // the box is taller than the text
	narrow := ts.layout_rich(runs, l.pieces[1].x + 10) // the box moves down whole
	assert narrow.lines.len == 3 // ab / the box / cd
	assert narrow.lines[1].pieces[0].width == 30
}

// A word wider than the line breaks inside: after a separator that fits, else anywhere.
fn test_long_words_break_inside() {
	mut ts := text.new_system(fake.new())
	pieces := fn (rt &text.RichText, s string) []string {
		mut out := []string{}
		for l in rt.lines {
			out << l.pieces.map(s[it.start..it.end]).join('')
		}
		return out
	}
	path := 'apps/folio/samples/welcome.md' // 6 px a character
	rt := ts.layout_rich([run(path, style(10, 400))], 90) // 15 characters a line
	assert pieces(rt, path) == ['apps/folio/', 'samples/', 'welcome.md']
	assert rt.width <= 90 && rt.width_breaks
	plain := 'abcdefghijklmnopqrstuvwxyz'
	rt2 := ts.layout_rich([run('see ' + plain, style(10, 400))], 60)
	assert pieces(rt2, 'see ' + plain) == ['see ', 'abcdefghij', 'klmnopqrst', 'uvwxyz']
}
