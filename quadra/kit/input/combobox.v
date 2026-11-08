// Combobox: an input whose typing filters a list of options shown below it, and the
// command palette built on the same matching.
module input

import quadra.core
import quadra.prim
import quadra.kit
import quadra.text
import quadra.ui

// fuzzy reports whether all characters of query appear in s in order (case-insensitive),
// and a score that prefers contiguous and early matches.
pub fn fuzzy(query string, s string) ?int {
	hits := fuzzy_hits(query, s)?
	mut score, mut last := 0, -2
	for i in hits {
		score += if i == last + 1 { 5 } else { 1 }
		if i == 0 {
			score += 3
		}
		last = i
	}
	return score
}

// fuzzy_hits returns the rune indices of s that match the query's characters in order
// (the first occurrence of each, ignoring case and diacritics), or none if one is missing.
pub fn fuzzy_hits(query string, s string) ?[]int {
	q := text.fold_diacritics(query).to_lower().runes()
	t := text.fold_diacritics(s).to_lower().runes()
	mut hits := []int{cap: q.len}
	for i, r in t {
		if hits.len < q.len && r == q[hits.len] {
			hits << i
		}
	}
	return if hits.len == q.len { hits } else { none }
}

// marked splits s into spans, the runes at hits in the color: a match highlight.
pub fn marked(s string, hits []int, color prim.Color) []ui.Span {
	runes := s.runes()
	mut spans := []ui.Span{}
	mut start, mut on := 0, false
	for i in 0 .. runes.len + 1 {
		hit := i < runes.len && i in hits
		if i == runes.len || hit != on {
			if i > start {
				part := runes[start..i].string()
				spans << if on { ui.span(part).color(color) } else { ui.span(part) }
			}
			start, on = i, hit
		}
	}
	return spans
}

@[heap]
pub struct Combobox {
pub mut:
	field     core.View[TextInput]
	options   []string
	matches   []int
	query     string
	highlight int
	open      bool
	picked    ?core.AnyListener // int: index into options
}

pub fn new_combobox(mut cx core.Cx, id string, options []string, placeholder string) core.View[Combobox] {
	field := new_input(mut cx, id: id, placeholder: placeholder, prefix: kit.icon_search)
	view := cx.new_view(Combobox{
		field:   field
		options: options
		matches: []int{len: options.len, init: index}
	})
	on_change(mut cx, field, core.Listener[string]{core.AnyListener{
		view: view.id
		call: combo_typed
	}})
	return view
}

pub fn on_pick(mut cx core.Cx, c core.View[Combobox], l core.Listener[int]) {
	mut w := cx.peek(c.model())
	w.picked = l.any
}

fn combo_typed(_ voidptr, view voidptr, event voidptr, mut _ core.Cx) {
	mut c := unsafe { &Combobox(view) }
	q := unsafe { *(&string(event)) }
	c.query = q
	mut scored := []prim.Point{} // x = score, y = index (keeps the sort simple)
	for i, o in c.options {
		if s := fuzzy(q, o) {
			scored << prim.pt(f32(s), f32(i))
		}
	}
	scored.sort_with_compare(fn (a &prim.Point, b &prim.Point) int {
		if a.x != b.x {
			return if a.x > b.x { -1 } else { 1 }
		}
		return int(a.y - b.y)
	})
	c.matches = scored.map(int(it.y))
	c.highlight = 0
	c.open = q != ''
}

fn (mut c Combobox) choose(mut cx core.ViewCx[Combobox]) {
	if c.highlight < c.matches.len {
		i := c.matches[c.highlight]
		set_value(mut cx.Cx, c.field, c.options[i])
		c.open = false
		if l := c.picked {
			cx.app.invoke(l, &i, cx.win, true)
		}
	}
}

@[markused]
pub fn (mut c Combobox) render(mut cx core.ViewCx[Combobox]) core.Element {
	t := kit.theme(cx)
	mut root := ui.div().flex_col().w(280).on_key_down(cx.listener(fn (mut c Combobox, e &core.KeyDownEvent, mut cx core.ViewCx[Combobox]) {
		n := c.matches.len
		match e.key {
			.down {
				c.open = true
				if n > 0 {
					c.highlight = (c.highlight + 1) % n
				}
			}
			.up {
				if n > 0 {
					c.highlight = (c.highlight - 1 + n) % n
				}
			}
			.enter {
				c.choose(mut cx)
			}
			.escape {
				c.open = false
			}
			else {
				return
			}
		}

		cx.stop_propagation()
	})).child(cx.child_view(c.field))
	if c.open && c.matches.len > 0 {
		mut list :=
			ui.div().flex_col().py(4).w(280).max_h(240).overflow_y_scroll().bg(t.p.overlay_bg).rounded(t.radius_xl).border(1).border_color(t.p.line).shadow_level(1).role(.listbox)
		for k, i in c.matches {
			on := k == c.highlight
			// Menu rows (specs/15): inset 4, padded 6, 23 tall.
			mut row := ui.div().test_id('opt-${i}').mx(4).px(6).h(23).flex_row().items_center().rounded(t.radius_md).cursor(.pointer).role(.listitem).label(c.options[i]).selected(on).on_click(core.listener_with[Combobox, int, core.ClickEvent](cx.entity,
				k, fn (mut c Combobox, k int, _ &core.ClickEvent, mut cx core.ViewCx[Combobox]) {
				c.highlight = k
				c.choose(mut cx)
			}))
			if on {
				row.bg(t.p.press)
			} else {
				row.hover(ui.Patch{
					background: t.p.panel_hover
				})
			}
			hits := fuzzy_hits(c.query, c.options[i]) or { []int{} }
			list.child(row.child(ui.rich_text(marked(c.options[i], hits, t.p.accent_text))))
		}
		root.child(ui.anchored(list).placement(.bottom_start).layer(50))
	}
	return root
}
