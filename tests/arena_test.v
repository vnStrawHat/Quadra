// Render arenas (specs/07, Element memory): a view's render reuses the elements of its render
// before last, reset; clean views keep their trees; elements made outside a render, or kept
// by a pinned render, are never reused.
module main

import quadra.core
import quadra.kit
import quadra.prim
import quadra.testing
import quadra.ui

struct Flip {
mut:
	n    int
	divs []voidptr // the root div of each render (test inspection only)
}

@[markused]
pub fn (mut f Flip) render(mut cx core.ViewCx[Flip]) core.Element {
	mut d := ui.div()
	if f.n % 2 == 0 {
		d = d.test_id('busy').bg(prim.rgb(0xff0000)).p(4).hover(ui.Patch{
			background: prim.rgb(0x00ff00)
		}).on_click(core.handler(fn (_ &core.ClickEvent, mut _ core.Cx) {})).child(ui.text('busy ${f.n}'))
	}
	f.divs << voidptr(d)
	return d
}

fn bump[T](mut h testing.Harness, v core.View[T], f fn (mut x T, mut cx core.ViewCx[T])) {
	mut cx := core.Cx{
		app: h.app
	}
	cx.update(v.model(), f)
	h.frame()
}

fn flip(mut f Flip, mut _ core.ViewCx[Flip]) {
	f.n++
}

fn test_recycled_element_is_reset() {
	assert core.render_arena() == unsafe { nil } // tests build on the heap
	mut h := testing.new_harness()!
	v := h.open(Flip{})!
	for _ in 0 .. 3 {
		bump(mut h, v, flip)
	}
	hc := core.Cx{
		app: h.app
	}
	f := hc.peek(v.model())
	assert f.divs.len == 4
	// renders alternate between the two arenas of the view
	assert f.divs[2] == f.divs[0]
	assert f.divs[3] == f.divs[1]
	assert f.divs[1] != f.divs[0]
	// render 3 is plain: the div that was busy in render 0, 2 is back to its defaults
	d := unsafe { &ui.Div(f.divs[3]) }
	assert d.test_id == ''
	assert d.children.len == 0
	assert d.handlers.len == 0
	assert d.states == unsafe { nil }
	assert d.visual.background == prim.Color{}
	assert d.style.padding == prim.Edges{}
	assert h.find('busy') == none
	bump(mut h, v, flip)
	assert h.has_text('busy 4')
	assert h.find('busy') != none
}

struct Still {
mut:
	root voidptr
}

@[markused]
pub fn (mut s Still) render(mut cx core.ViewCx[Still]) core.Element {
	d := ui.div().test_id('still').h(30).bg(prim.rgb(0x123456)).child(ui.text('still here'))
	s.root = voidptr(d)
	return d
}

struct Pair {
	still core.View[Still]
	flip  core.View[Flip]
}

@[markused]
pub fn (mut p Pair) render(mut cx core.ViewCx[Pair]) core.Element {
	return ui.div().flex_col().child(cx.child_view(p.still)).child(cx.child_view(p.flip))
}

fn test_clean_view_keeps_its_tree_while_others_rerender() {
	mut h := testing.new_harness()!
	mut cx := core.Cx{
		app: h.app
	}
	still := cx.new_view(Still{})
	fl := cx.new_view(Flip{})
	h.open(Pair{ still: still, flip: fl })!
	root := cx.peek(still.model()).root
	for _ in 0 .. 9 {
		bump(mut h, fl, flip)
	}
	assert h.renders_of(still) == 1
	assert h.renders_of(fl) == 10
	d := unsafe { &ui.Div(root) }
	assert d.test_id == 'still'
	assert d.children.len == 1
	assert h.has_text('still here')
	assert h.find('still') != none
}

struct Keeps {
mut:
	n    int
	kept &ui.Div = unsafe { nil } // made outside any render
	held core.Model[Held]
}

struct Held {
	el &ui.TextEl = unsafe { nil }
}

@[markused]
pub fn (mut k Keeps) render(mut cx core.ViewCx[Keeps]) core.Element {
	if k.held.id == 0 {
		// An element stored in a model made during render: the render is pinned.
		k.held = cx.new_model(Held{
			el: ui.text('pinned text')
		})
	}
	held := cx.read(k.held)
	return ui.div().flex_col().child(k.kept).child(held.el).child(ui.text('n ${k.n}'))
}

fn keep_bump(mut k Keeps, mut _ core.ViewCx[Keeps]) {
	k.n++
}

fn test_elements_kept_outside_a_render_stay_valid() {
	mut h := testing.new_harness()!
	kept := ui.div().test_id('kept').child(ui.text('made outside'))
	v := h.open(Keeps{ kept: kept })!
	for _ in 0 .. 8 {
		bump(mut h, v, keep_bump)
	}
	assert h.has_text('n 8')
	assert h.has_text('made outside')
	assert h.has_text('pinned text')
	assert kept.test_id == 'kept'
	assert h.find('kept') != none
}

fn test_number_formats_like_interpolation() {
	for x in [0.0, 1.0, -1.0, 0.5, 2.675, 999.995, 1234.5678, -0.004, -12.345, 1.0e9, 0.1] {
		assert ui.number(x, 2).content == '${x:.2f}', '${x}'
		assert ui.number(x, 0).content == '${x:.0f}', '${x}'
		assert ui.number(x, 3).content == '${x:.3f}', '${x}'
	}
	for x in [-0.0, 1.0e15, 123456789.125, 0.0049999, 0.005] {
		assert ui.number(x, 2).content == '${x:.2f}', '${x}'
	}
	for i in 0 .. 100000 {
		x := f64(i) / 100
		assert ui.number(x, 2).content == '${x:.2f}'
		y := f64(i) * 0.00731 - 300
		assert ui.number(y, 3).content == '${y:.3f}', '${y}'
		assert ui.number(y, 1).content == '${y:.1f}', '${y}'
	}
}

struct Built {
mut:
	n int
}

struct MenuSrc {
	built &Built = unsafe { nil }
}

fn lazy_items(s MenuSrc) []kit.MenuItem {
	mut b := s.built
	b.n++
	return [kit.MenuItem{
		label: 'Lazy item'
	}]
}

struct LazyMenuView {
	src MenuSrc
}

@[markused]
pub fn (mut v LazyMenuView) render(mut cx core.ViewCx[LazyMenuView]) core.Element {
	return ui.div().child(kit.menu_button_with('lazy-menu', 'Menu', v.src, lazy_items))
}

fn test_lazy_menu_builds_items_when_it_opens() {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	mut built := &Built{}
	v := h.open(LazyMenuView{ src: MenuSrc{built} })!
	for _ in 0 .. 3 {
		mut cx := core.Cx{
			app:    h.app
			entity: v.id
		}
		cx.notify()
		h.frame()
	}
	assert built.n == 0
	h.click('lazy-menu')!
	h.frame()
	assert built.n == 1
	assert h.has_text('Lazy item')
}
