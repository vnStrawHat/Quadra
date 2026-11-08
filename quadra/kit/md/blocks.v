// Markdown blocks as elements (view.v builds the column).
module md

import os
import quadra.core
import quadra.kit
import quadra.kit.code
import quadra.layout
import quadra.ui

// block renders one block. An if chain, not a match: values computed inside the arms of a
// match typed as core.Element get that type too (a V 0.5.2 bug).
fn (mut v MarkdownView) block(b Block, t &kit.Theme) core.Element {
	if b.kind == .heading {
		return v.heading(b, t)
	}
	if b.kind in [.bullet, .ordered] {
		return v.item(b, t)
	}
	if b.kind == .code {
		if b.lang == 'mermaid' {
			return v.diagram_block(b.text, t)
		}
		return code_box(code_spans(b.text, b.lang, t), t)
	}
	if b.kind == .math {
		return v.math_block(b.text, t)
	}
	if b.kind == .front {
		return code_box(code_spans(b.text, 'yaml', t), t)
	}
	if b.kind == .quote {
		mut quote :=
			ui.div().pl(f32(b.level - 1) * 16).child(ui.div().border_l(4).border_color(t.p.line_strong).pl(16).text_color(t.p.text_2).child(v.rich(v.spans(b.text, t))))
		return quote
	}
	if b.kind == .rule {
		return ui.div().h(1).w_full().bg(t.p.line_strong)
	}
	if b.kind == .table {
		return v.table(b, t)
	}
	if img := v.image_only(b.text) {
		return img
	}
	return v.rich(v.spans(b.text, t))
}

fn (mut v MarkdownView) heading(b Block, t &kit.Theme) core.Element {
	// 28, 22.4, 19.2, 16, 14, 13.6 at 16 px body, semibold, 8 more space above (specs/15).
	scale := match b.level {
		1 { f32(1.75) }
		2 { f32(1.4) }
		3 { f32(1.2) }
		4 { f32(1.0) }
		5 { f32(0.875) }
		else { f32(0.85) }
	}

	mut s := v.spans(b.text, t)
	for mut sp in s {
		sp = sp.weight(600)
	}
	text := plain(b.text)
	mut h := ui.div().test_id('h-${anchor(text)}').role(.heading).label(text).pt(8).line_height(1.25).child(v.rich(s).size(v.body_size(t) * scale).color(if b.level == 6 {
		t.p.text_2
	} else {
		t.p.text
	}))
	if b.level <= 2 {
		h.pb(if b.level == 1 { 8 } else { 4 }).border_b(1).border_color(t.p.line)
	}
	return h
}

fn (mut v MarkdownView) item(b Block, t &kit.Theme) core.Element {
	body := ui.div().grow().child(v.rich(v.spans(b.text, t)).color(if b.task == 2 {
		t.p.text_3
	} else {
		t.p.text
	}))
	mut mark := ui.div().w(22).shrink_0().flex_row().items_center().h(v.body_size(t) * 1.5)
	if b.task > 0 { // a checkbox (specs/15): outlined, an accent check
		box :=
			ui.div().size(16).rounded(t.radius_sm).border(1).border_color(t.p.line_strong).center()
		if b.task == 2 {
			box.child(kit.icon(kit.icon_check).size(14).color(t.p.accent_text))
		}
		mark.child(box)
	} else {
		mark.child(ui.text(list_marker(b)).color(t.p.text_3))
	}
	return ui.div().flex_row().pl(f32(b.level) * 22).child(mark).child(body)
}

// code_box is a code block (specs/15): the editor tone, a hairline, radius 6, padding 12,
// the code face at the editor's size and line height.
fn code_box(spans []ui.Span, t &kit.Theme) core.Element {
	return ui.div().p(12).rounded(t.radius_lg).bg(t.p.canvas).border(1).border_color(t.p.line).overflow_x_scroll().line_height(t.mono_line).child(ui.rich_text(spans).size(t.scaled(14)))
}

// code_spans colors code with the tokenizer for its language.
fn code_spans(src string, lang string, t &kit.Theme) []ui.Span {
	tk := code.tokenizer_for('x.${lang}')
	mut out := []ui.Span{}
	mut state := 0
	for k, raw in src.split('\n') {
		line := raw.replace('\t', '    ')
		if k > 0 {
			out << ui.span('\n').family(t.mono_family)
		}
		toks, next := tk.tokenize(line, state)
		state = next
		mut at := 0
		for tok in toks {
			if tok.start > at {
				out << ui.span(line[at..tok.start]).family(t.mono_family)
			}
			out << ui.span(line[tok.start..tok.end]).family(t.mono_family).color(code.syntax_color(&t.code.syntax,
				tok.kind))
			at = tok.end
		}
		if at < line.len {
			out << ui.span(line[at..]).family(t.mono_family)
		}
	}
	return out
}

fn (mut v MarkdownView) table(b Block, t &kit.Theme) core.Element {
	mut grid :=
		ui.div().flex_col().border(1).border_color(t.p.line_strong).rounded(t.radius_md).overflow_hidden()
	// A bordered grid (specs/15): the header row on the chrome tone, odd rows on the panel.
	for r, row in b.rows {
		mut line := ui.div().flex_row()
		if r == 0 {
			line.bg(t.p.chrome)
		} else {
			line.border_t(1).border_color(t.p.line_strong)
			if r % 2 == 0 {
				line.bg(t.p.panel)
			}
		}
		for c, cell in row {
			mut s := v.spans(cell, t)
			if r == 0 {
				for mut sp in s {
					sp = sp.weight(600)
				}
			}
			mut box := ui.div().flex(1, 1, layout.px(0)).flex_row().px(10).py(4)
			al := if c < b.align.len { b.align[c] } else { Align.none }
			if al == .center {
				box.justify_center()
			} else if al == .right {
				box.justify_end()
			}
			line.child(box.child(v.rich(s)))
		}
		grid.child(line)
	}
	return grid
}

// image_only shows a paragraph that is just an image as the image, scaled to the column.
fn (v &MarkdownView) image_only(text string) ?core.Element {
	sp := parse_inline(text.trim_space())
	if sp.len != 1 || sp[0].image == '' {
		return none
	}
	src := sp[0].image
	if src.starts_with('http://') || src.starts_with('https://') {
		return none // no network: the alt text and url show as text
	}
	path := if os.is_abs_path(src) { src } else { os.join_path(v.base, src) }
	if !os.is_file(path) {
		return none
	}
	return ui.div().w_full().child(ui.img(path).fit(.contain).rounded(4))
}

fn list_marker(b Block) string {
	if b.kind == .ordered {
		return '${b.num}.'
	}
	return if b.level == 0 { '•' } else { '◦' }
}
