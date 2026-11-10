// The search bar (specs/22): one per terminal, drawn over its grid.
module main

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.kit.term
import quadra.ui

// FindBar is a terminal's search bar; the query and matches are the terminal's own Finder.
struct FindBar {
mut:
	open  bool
	input core.View[input.TextInput]
}

fn ext_of(t &term.Terminal) &TermExt {
	return unsafe { &TermExt(t.hooks.holder) }
}

// toggle_find opens the bar of the active terminal, or closes it when open.
fn (mut o OneTerm) toggle_find(mut cx core.ViewCx[OneTerm]) {
	tab := o.current()
	for s in tab.spaces {
		if s.id == tab.active && s.view.id != 0 {
			cx.update_view(s.view, fn (mut t term.Terminal, mut cx core.ViewCx[term.Terminal]) {
				mut x := ext_of(t)
				x.toggle_find(mut t, mut cx)
			})
		}
	}
}

fn (mut x TermExt) toggle_find(mut t term.Terminal, mut cx core.ViewCx[term.Terminal]) {
	if x.find.open {
		x.close_find(mut t, mut cx)
		return
	}
	if x.find.input.id == 0 {
		x.find.input = input.new_input(mut cx.Cx,
			id:          'find-input-${t.id}'
			placeholder: 'Find in terminal'
			bare:        true
		)
		input.on_change(mut cx.Cx, x.find.input, cx.listener(fn (mut t term.Terminal, v &string, mut cx core.ViewCx[term.Terminal]) {
			t.find.query = *v
			cx.debounce('find', 150, fn (mut t term.Terminal, mut cx core.ViewCx[term.Terminal]) {
				t.search(t.find.query, t.find.case_sensitive, t.find.whole_word)
				cx.notify()
			})
		}))
	} else {
		input.set_value(mut cx.Cx, x.find.input, '')
	}
	x.find.open = true
	cx.peek(x.find.input.model()).focus.focus(mut cx.Cx)
	cx.notify()
}

fn (mut x TermExt) close_find(mut t term.Terminal, mut cx core.ViewCx[term.Terminal]) {
	x.find.open = false
	t.search_clear()
	t.focus.focus(mut cx.Cx)
	cx.notify()
}

fn find_step(mut t term.Terminal, delta int, mut cx core.ViewCx[term.Terminal]) {
	t.search_step(delta)
	cx.notify()
}

// find_label is the counter: nothing while no query is set, n/total with no active match
// counting as the first.
fn find_label(t &term.Terminal) string {
	f := t.find
	if f.query == '' {
		return ''
	}
	if f.hits.len == 0 {
		return 'No matches'
	}
	return '${(if f.current >= 0 {
		f.current
	} else {
		0
	}) + 1}/${f.hits.len}'
}

fn find_key(mut t term.Terminal, e &core.KeyDownEvent, mut cx core.ViewCx[term.Terminal]) {
	if e.key == .enter {
		find_step(mut t, if e.mods.has(.shift) { -1 } else { 1 }, mut cx)
	}
	cx.stop_propagation() // the bar owns the keyboard: nothing reaches the program
}

fn run_find_now(mut t term.Terminal, mut cx core.ViewCx[term.Terminal]) {
	t.search(t.find.query, t.find.case_sensitive, t.find.whole_word)
	cx.notify()
}

fn (x &TermExt) find_bar(t &term.Terminal, th &kit.Theme, mut cx core.ViewCx[term.Terminal]) core.Element {
	case_btn := kit.toggle_button('find-case', t.find.case_sensitive).label('Aa').size(.xs).tooltip('Match case').on_toggle(cx.listener(fn (mut t term.Terminal, v &bool, mut cx core.ViewCx[term.Terminal]) {
		t.find.case_sensitive = *v
		run_find_now(mut t, mut cx)
	}))
	word_btn := kit.toggle_button('find-word', t.find.whole_word).label('W').size(.xs).tooltip('Match whole word').on_toggle(cx.listener(fn (mut t term.Terminal, v &bool, mut cx core.ViewCx[term.Terminal]) {
		t.find.whole_word = *v
		run_find_now(mut t, mut cx)
	}))
	prev_btn := kit.icon_button('find-prev', kit.icon_arrow_up).ghost().size(.xs).tooltip('Previous match (Shift+Enter)').on_click(cx.listener(fn (mut t term.Terminal, _ &core.ClickEvent, mut cx core.ViewCx[term.Terminal]) {
		find_step(mut t, -1, mut cx)
	}))
	next_btn := kit.icon_button('find-next', kit.icon_arrow_down).ghost().size(.xs).tooltip('Next match (Enter)').on_click(cx.listener(fn (mut t term.Terminal, _ &core.ClickEvent, mut cx core.ViewCx[term.Terminal]) {
		find_step(mut t, 1, mut cx)
	}))
	close_btn := kit.icon_button('find-close', kit.icon_x).ghost().size(.xs).tooltip('Close (Esc)').on_click(cx.listener(fn (mut t term.Terminal, _ &core.ClickEvent, mut cx core.ViewCx[term.Terminal]) {
		mut x := ext_of(t)
		x.close_find(mut t, mut cx)
	}))
	label := find_label(t)
	label_color := if t.find.query != '' && t.find.hits.len == 0 { th.p.text_2 } else { th.p.text }
	counter :=
		ui.div().min_w(th.sp(76)).px(th.sp(4)).center().child(ui.text(label).size(th.font_size(.xs)).color(label_color))
	divider := ui.div().w(1).h(18).bg(th.p.line)
	return ui.div().id('find-bar').test_id('find-bar').absolute().top(8).right(8).w(360).flex_row().items_center().gap(th.sp(4)).px(th.sp(6)).py(th.sp(4)).bg(th.p.canvas.alpha(0.97)).border(1).border_color(th.p.line).rounded(6).shadow_level(1).on_action(cx.listener(fn (mut t term.Terminal, _ &kit.Dismiss, mut cx core.ViewCx[term.Terminal]) {
		mut x := ext_of(t)
		x.close_find(mut t, mut cx)
	})).on_key_down(cx.listener(fn (mut t term.Terminal, e &core.KeyDownEvent, mut cx core.ViewCx[term.Terminal]) {
		find_key(mut t, e, mut cx)
	})).on_text_input(cx.listener(fn (mut _ term.Terminal, _ &core.TextInputEvent, mut cx core.ViewCx[term.Terminal]) {
		cx.stop_propagation()
	})).on_mouse_down(cx.listener(fn (mut _ term.Terminal, _ &core.MouseDownEvent, mut cx core.ViewCx[term.Terminal]) {
		cx.stop_propagation() // the bar floats over the grid: clicks must not start a selection
	})).on_mouse_up(cx.listener(fn (mut _ term.Terminal, _ &core.MouseUpEvent, mut cx core.ViewCx[term.Terminal]) {
		cx.stop_propagation()
	})).child(case_btn).child(word_btn).child(divider).child(ui.div().grow().min_w(120).child(core.view_element(x.find.input.any()))).child(counter).child(prev_btn).child(next_btn).child(close_btn)
}
