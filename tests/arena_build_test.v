// Element arenas of layout builds and nested renders (specs/07, Element memory): what a kit
// component builds during layout reuses the elements of the layout before last, and a view
// rendered inside a build reuses its own; keep_build pins a pass.
module main

import quadra.core
import quadra.kit
import quadra.layout
import quadra.prim
import quadra.testing
import quadra.ui

// Probe is a kit-style component: it builds a div at every layout, records it, and lays out
// the view inside it while the build runs (a nested render).
@[heap]
struct Probe {
	kit.Lazy
	inner ?core.AnyView
mut:
	seen []voidptr
	keep bool // call keep_build once, after laying out the inner view
}

pub fn (mut p Probe) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut p, mut cx)
	return p.layout_inner(el, mut cx)
}

pub fn (mut p Probe) build_tree(mut cx core.LayoutCx) core.Element {
	d := ui.div().test_id('probe').bg(prim.rgb(0x204060)).hover(ui.Patch{
		background: prim.rgb(0x406080)
	}).child(ui.text('probe text'))
	p.seen << voidptr(d)
	if v := p.inner {
		mut el := core.view_element(v)
		el.request_layout(mut cx) // renders the view now, inside this build
		d.child(el)
		if p.keep {
			p.keep = false
			mut kcx := core.Cx{
				app: cx.app
				win: cx.win
			}
			kcx.keep_build() // an overlay now holds this build, the inner view's tree included
		}
	}
	return d
}

struct Inner {
mut:
	n    int
	divs []voidptr
}

@[markused]
pub fn (mut i Inner) render(mut cx core.ViewCx[Inner]) core.Element {
	d := ui.div().child(ui.text('inner ${i.n}'))
	i.divs << voidptr(d)
	return d
}

struct Outer {
	probe &Probe
}

@[markused]
pub fn (mut o Outer) render(mut cx core.ViewCx[Outer]) core.Element {
	return ui.div().size_full().child(o.probe)
}

fn rebuild(mut h testing.Harness) {
	mut w := h.window()
	w.invalidate()
	h.frame()
}

fn test_layout_builds_reuse_the_pass_before_last() {
	mut h := testing.new_harness()!
	mut probe := &Probe{}
	h.open(Outer{probe}, width: 400, height: 300)!
	for _ in 0 .. 4 {
		rebuild(mut h)
	}
	// opening the window pinned the first pass (it made views); then passes alternate
	assert probe.seen.len == 5
	assert probe.seen[3] == probe.seen[1]
	assert probe.seen[4] == probe.seen[2]
	assert probe.seen[2] != probe.seen[1]
	assert h.has_text('probe text')
	assert h.find('probe') != none
	// keep_build (an overlay opened from a build) pins the pass it was called after: what
	// that pass built is never handed out again.
	mut cx := core.Cx{
		app: h.app
		win: h.window()
	}
	cx.keep_build()
	rebuild(mut h)
	rebuild(mut h)
	rebuild(mut h)
	assert probe.seen[4] !in probe.seen[5..]
	assert h.has_text('probe text')
}

fn bump_inner(mut i Inner, mut _ core.ViewCx[Inner]) {
	i.n++
}

fn test_view_rendered_inside_a_build_uses_its_arena() {
	mut h := testing.new_harness()!
	mut cx := core.Cx{
		app: h.app
	}
	inner := cx.new_view(Inner{})
	mut probe := &Probe{
		inner: inner.any()
	}
	h.open(Outer{probe}, width: 400, height: 300)!
	for _ in 0 .. 4 {
		cx.update(inner.model(), bump_inner)
		h.frame()
	}
	i := cx.peek(inner.model())
	assert i.divs.len == 5
	assert i.divs[3] == i.divs[1]
	assert i.divs[4] == i.divs[2]
	assert i.divs[2] != i.divs[1]
	assert h.has_text('inner 4')
}

// keep_build from a build pins the arenas of the views rendered inside it: the inner view's
// tree held at that moment is never reset and handed out again.
fn test_keep_build_pins_views_rendered_inside_the_build() {
	mut h := testing.new_harness()!
	mut cx := core.Cx{
		app: h.app
	}
	inner := cx.new_view(Inner{})
	mut probe := &Probe{
		inner: inner.any()
	}
	h.open(Outer{probe}, width: 400, height: 300)!
	for _ in 0 .. 2 {
		cx.update(inner.model(), bump_inner)
		h.frame()
	}
	probe.keep = true
	cx.update(inner.model(), bump_inner)
	h.frame()
	held := cx.peek(inner.model()).divs.last()
	for _ in 0 .. 4 {
		cx.update(inner.model(), bump_inner)
		h.frame()
	}
	i := cx.peek(inner.model())
	assert i.divs.len == 8
	assert held !in i.divs[6..]
	assert i.divs[7] == i.divs[5] // reuse resumes after the pinned trees are dropped
	assert h.has_text('inner 7')
}
