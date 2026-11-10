// Math and Mermaid in the Markdown view. Diagrams are kept between builds by their source
// (Memo), so typing elsewhere in the document does not parse and lay them out again.
module md

import quadra.core
import quadra.kit
import quadra.kit.md.mermaid
import quadra.kit.md.tex
import quadra.ui

// Memo keeps the diagrams of the last build, keyed by source and occurrence (the same
// source twice is two elements). The app owns it and passes it to memo().
@[heap]
pub struct Memo {
mut:
	diagrams map[string]&mermaid.Diagram
	formulas map[string]&tex.Formula
	used     map[string]bool
	seen     map[string]int // occurrences in the current build
}

// start begins a build.
fn (mut m Memo) start() {
	m.used.clear()
	m.seen.clear()
}

// finish drops what the build did not use.
fn (mut m Memo) finish() {
	for k in m.diagrams.keys() {
		if k !in m.used {
			m.diagrams.delete(k)
		}
	}
	for k in m.formulas.keys() {
		if k !in m.used {
			m.formulas.delete(k)
		}
	}
}

fn (mut m Memo) key(src string) string {
	n := m.seen[src]
	m.seen[src] = n + 1
	k := '${n}\x00${src}'
	m.used[k] = true
	return k
}

fn (mut v MarkdownView) diagram(src string) &mermaid.Diagram {
	if v.memo == unsafe { nil } {
		return mermaid.diagram(src)
	}
	k := v.memo.key(src)
	if d := v.memo.diagrams[k] {
		return d
	}
	d := mermaid.diagram(src)
	v.memo.diagrams[k] = d
	return d
}

fn (mut v MarkdownView) diagram_block(src string, t &kit.Theme) core.Element {
	d := v.diagram(src)
	if msg := d.error() {
		note := ui.text('Diagram: ${msg}').size(t.ui_text(.sm)).color(t.p.text_3)
		return ui.div().flex_col().gap(6).child(v.code_box(code_spans(src, 'mermaid', t), t)).child(note)
	}
	return ui.div().w_full().overflow_x_scroll().flex_row().justify_center().py(8).child(d)
}

fn (mut v MarkdownView) formula(src string, display bool) &tex.Formula {
	if v.memo == unsafe { nil } {
		return tex.formula(src, display)
	}
	k := v.memo.key(if display { 'D' } else { 'I' } + src)
	if f := v.memo.formulas[k] {
		return f
	}
	f := tex.formula(src, display)
	v.memo.formulas[k] = f
	return f
}

fn (mut v MarkdownView) math_block(src string, t &kit.Theme) core.Element {
	f := v.formula(src, true)
	if msg := f.error() {
		note := ui.text('Math: ${msg}').size(t.ui_text(.sm)).color(t.p.text_3)
		return ui.div().flex_col().gap(6).child(v.code_box(code_spans(src, 'tex', t), t)).child(note)
	}
	return ui.div().w_full().overflow_x_scroll().flex_row().justify_center().py(6).child(f)
}

fn (mut v MarkdownView) inline_math(src string) ui.Span {
	return ui.inline_box(v.formula(src, false))
}
