// The find bar: an inline panel at the top right of the editor with the query, the match
// options, "n of m", previous / next and (ctrl-h) replace and replace all.
module code

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui

fn (mut e Editor) open_find(replace bool, mut cx core.ViewCx[Editor]) {
	if e.query == none {
		q := input.new_input(mut cx.Cx, id: '${e.id}-find', placeholder: 'Find')
		input.on_change(mut cx.Cx, q, cx.listener(fn (mut e Editor, s &string, mut cx core.ViewCx[Editor]) {
			e.find.query = *s
			e.search_from_cursor()
			cx.notify()
		}))
		input.on_submit(mut cx.Cx, q, cx.listener(fn (mut e Editor, s &string, mut cx core.ViewCx[Editor]) {
			e.find.next(mut e.doc, false)
			e.edited(e.doc.version, mut cx)
		}))
		r := input.new_input(mut cx.Cx, id: '${e.id}-replace', placeholder: 'Replace')
		input.on_submit(mut cx.Cx, r, cx.listener(fn (mut e Editor, s &string, mut cx core.ViewCx[Editor]) {
			e.replace_one(mut cx)
		}))
		e.query = q
		e.replace_with = r
	}
	q := e.query or { return }
	// Seed the query with a one-line selection.
	p := e.doc.primary()
	if !p.empty() && e.doc.buf.line_of(p.start()) == e.doc.buf.line_of(p.end()) {
		e.find.query = e.doc.buf.slice(p.start(), p.end())
		input.set_value(mut cx.Cx, q, e.find.query)
	}
	e.find_open = true
	e.show_replace = replace && !e.readonly
	cx.peek(q.model()).focus.focus(mut cx.Cx)
}

fn (mut e Editor) close_find(mut cx core.ViewCx[Editor]) {
	e.find_open = false
	e.focus.focus(mut cx.Cx)
}

// search_from_cursor selects the first match at or after the cursor as the query changes.
fn (mut e Editor) search_from_cursor() {
	hits := e.find.matches(e.doc)
	if hits.len == 0 {
		return
	}
	from := e.doc.primary().start()
	mut k := 0
	for k + 1 < hits.len && hits[k] < from {
		k += 2
	}
	if k >= hits.len {
		k = 0
	}
	e.doc.set_cursor(hits[k], hits[k + 1], false)
	e.reveal = true
}

fn (mut e Editor) replace_one(mut cx core.ViewCx[Editor]) {
	r := e.replace_with or { return }
	v := e.doc.version
	e.find.replace(mut e.doc, cx.peek(r.model()).value())
	e.edited(v, mut cx)
}

fn (mut e Editor) replace_all(mut cx core.ViewCx[Editor]) {
	r := e.replace_with or { return }
	v := e.doc.version
	e.find.replace_all(mut e.doc, cx.peek(r.model()).value())
	e.edited(v, mut cx)
}

// status is the "n of m" label.
fn (mut e Editor) status() string {
	if e.find.error() != '' {
		return 'Bad pattern'
	}
	n := e.find.matches(e.doc).len / 2
	if e.find.query == '' {
		return ''
	}
	if n == 0 {
		return 'No results'
	}
	cur := e.find.current(e.doc)
	return if cur >= 0 { '${cur + 1} of ${n}' } else { '${n} found' }
}

fn (mut e Editor) option(id string, label string, on bool, tip string, mut cx core.ViewCx[Editor]) &kit.Button {
	return kit.toggle_button('${e.id}-${id}', on).label(label).size(.sm).tooltip(tip).on_toggle(core.listener_with[Editor, string, bool](cx.entity,
		id, fn (mut e Editor, id string, v &bool, mut cx core.ViewCx[Editor]) {
		match id {
			'case' { e.find.case = *v }
			'word' { e.find.word = *v }
			else { e.find.regex = *v }
		}

		e.search_from_cursor()
		cx.notify()
	}))
}

fn (mut e Editor) find_bar(mut cx core.ViewCx[Editor]) core.Element {
	t := kit.theme(cx)
	q := e.query or { return ui.div() }
	r := e.replace_with or { return ui.div() }
	mut row := ui.div().flex_row().items(.center).gap(2).child(ui.div().w(220).flex_col().child(cx.child_view(q))).child(e.option('case',
		'Aa', e.find.case, 'Match case', mut cx)).child(e.option('word', 'ab', e.find.word,
		'Whole word', mut cx)).child(e.option('regex', '.*', e.find.regex, 'Regular expression', mut cx)).child(ui.div().w(76).px(6).child(ui.text(e.status()).size(t.ui_text(.sm)).color(t.p.text_2))).child(kit.icon_button('${e.id}-prev',
		kit.icon_arrow_up).size(.sm).tooltip('Previous match').on_click(cx.listener(fn (mut e Editor, _ &core.ClickEvent, mut cx core.ViewCx[Editor]) {
		e.run(&EditorAction{ op: .find_prev }, mut cx)
	}))).child(kit.icon_button('${e.id}-next', kit.icon_arrow_down).size(.sm).tooltip('Next match').on_click(cx.listener(fn (mut e Editor, _ &core.ClickEvent, mut cx core.ViewCx[Editor]) {
		e.run(&EditorAction{ op: .find_next }, mut cx)
	}))).child(kit.icon_button('${e.id}-close', kit.icon_x).size(.sm).tooltip('Close').on_click(cx.listener(fn (mut e Editor, _ &core.ClickEvent, mut cx core.ViewCx[Editor]) {
		e.close_find(mut cx)
		cx.notify()
	})))
	mut bar :=
		ui.div().id('${e.id}-findbar').test_id('${e.id}-findbar').absolute().top(8).right(20).flex_col().gap(4).p(4).bg(t.p.overlay_bg).border(1).border_color(t.p.line).rounded(t.radius_xl).shadow_level(1).cursor(.arrow).child(row)
	// Clicks on the bar must not reach the editor (it would take the focus back).
	bar.on_mouse_down(cx.listener(fn (mut e Editor, _ &core.MouseDownEvent, mut cx core.ViewCx[Editor]) {
		cx.stop_propagation()
	}))
	if e.show_replace {
		bar.child(ui.div().flex_row().items(.center).gap(2).child(ui.div().w(220).flex_col().child(cx.child_view(r))).child(kit.button('${e.id}-replace-one').label('Replace').size(.sm).on_click(cx.listener(fn (mut e Editor, _ &core.ClickEvent, mut cx core.ViewCx[Editor]) {
			e.replace_one(mut cx)
		}))).child(kit.button('${e.id}-replace-all').label('All').size(.sm).on_click(cx.listener(fn (mut e Editor, _ &core.ClickEvent, mut cx core.ViewCx[Editor]) {
			e.replace_all(mut cx)
		}))))
	}
	return bar
}
