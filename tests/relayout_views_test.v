// Incremental layout with views nested in unusual ways: a view whose tree is another view.
module main

import quadra.core
import quadra.testing
import quadra.ui

struct Leaf {
mut:
	label string
}

@[markused]
pub fn (mut l Leaf) render(mut cx core.ViewCx[Leaf]) core.Element {
	return ui.div().test_id('leaf').px(4).child(ui.text(l.label))
}

// Wrap's tree is its child view's tree: both views share one root node.
struct Wrap {
mut:
	inner core.View[Leaf]
	n     int
}

@[markused]
pub fn (mut w Wrap) render(mut cx core.ViewCx[Wrap]) core.Element {
	return cx.child_view(w.inner)
}

struct Page {
mut:
	wrap core.View[Wrap]
}

@[markused]
pub fn (mut p Page) render(mut cx core.ViewCx[Page]) core.Element {
	return ui.div().flex_col().child(ui.text('above')).child(cx.child_view(p.wrap)).child(ui.div().test_id('below').h(10))
}

fn bump(mut w Wrap, _ int, mut _ core.ViewCx[Wrap]) {
	w.n++
}

fn relabel(mut l Leaf, s string, mut _ core.ViewCx[Leaf]) {
	l.label = s
}

fn check(h &testing.Harness) {
	if d := h.window().verify_layout() {
		assert false, d
	}
}

fn test_a_view_whose_tree_is_another_view() {
	mut h := testing.new_harness()!
	mut cx := core.Cx{
		app: h.app
	}
	leaf := cx.new_view(Leaf{
		label: 'one'
	})
	wrap := cx.new_view(Wrap{
		inner: leaf
	})
	h.open(Page{ wrap: wrap })!
	w := h.window()
	n := w.stats.relayouts
	cx.update_with(wrap.model(), 0, bump) // renders the same child: its node is placed again
	h.frame()
	check(h)
	cx.update_with(leaf.model(), 'a longer label', relabel) // wider: the page lays out again
	h.frame()
	check(h)
	cx.update_with(leaf.model(), 'two', relabel)
	cx.update_with(wrap.model(), 0, bump) // both at once
	h.frame()
	check(h)
	assert w.stats.relayouts == n + 3
	assert h.has_text('two')
	l := h.find('leaf') or { panic('') }
	b := h.find('below') or { panic('') }
	assert b.y == l.y + l.height
}
