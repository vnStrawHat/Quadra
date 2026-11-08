// Workbench components in the harness: tab strip, activity bar, pane tree.
module main

import quadra.core
import quadra.kit
import quadra.kit.work
import quadra.platform
import quadra.testing
import quadra.ui

struct Bench {
mut:
	tabs     []work.Tab = [work.Tab{'a', 'app.v', '', false}, work.Tab{'b', 'main.v', '', true},
		work.Tab{'c', 'README.md', '', false}]
	active   string        = 'a'
	activity string        = 'files'
	root     work.PaneNode = work.hsplit(work.leaf('left'), work.leaf('right'), 0.5)
}

@[markused]
pub fn (mut b Bench) render(mut cx core.ViewCx[Bench]) core.Element {
	strip := work.tab_strip('ts', b.tabs, b.active).on_select(cx.listener(fn (mut b Bench, id &string, mut _ core.ViewCx[Bench]) {
		b.active = *id
	})).on_close(cx.listener(fn (mut b Bench, id &string, mut _ core.ViewCx[Bench]) {
		b.tabs = b.tabs.filter(it.id != *id)
	})).on_move(cx.listener(fn (mut b Bench, m &work.TabMove, mut _ core.ViewCx[Bench]) {
		i := b.tabs.map(it.id).index(m.id)
		t := b.tabs[i]
		b.tabs.delete(i)
		b.tabs.insert(m.to, t)
	}))
	bar := work.activity_bar('ab', [work.Activity{'files', kit.icon_file, 'Files', 0},
		work.Activity{'search', kit.icon_search, 'Search', 3}], b.activity).on_select(cx.listener(fn (mut b Bench, id &string, mut _ core.ViewCx[Bench]) {
		b.activity = *id
	}))
	panes := work.panes('pz', b.root, {
		'left':  core.Element(ui.text('left'))
		'right': core.Element(ui.text('right'))
	}).on_resize(cx.listener(fn (mut b Bench, r &work.PaneResize, mut _ core.ViewCx[Bench]) {
		b.root = b.root.with_ratio(r.path, r.ratio)
	}))
	return ui.div().flex_row().size_full().child(bar).child(ui.div().flex_col().grow().child(strip).child(ui.div().grow().child(panes)))
}

fn open() !(&testing.Harness, core.View[Bench]) {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(Bench{}, width: 800, height: 400)!
	return h, v
}

fn bench_of(h &testing.Harness, v core.View[Bench]) &Bench {
	mut cx := core.Cx{
		app: h.app
	}
	return cx.peek(v.model())
}

fn mouse(mut h testing.Harness, a platform.MouseAction, b platform.MouseButton, x f32, y f32) {
	h.inject(platform.MouseEvent{ action: a, button: b, x: x, y: y })
}

fn test_select_close_and_middle_click() {
	mut h, v := open()!
	h.click('ts-c')!
	assert bench_of(h, v).active == 'c'
	h.click('ts-a-close')!
	assert bench_of(h, v).tabs.map(it.id) == ['b', 'c']
	assert bench_of(h, v).active == 'c' // closing does not select
	r := h.find('ts-b') or { panic('') }
	mouse(mut h, .down, .middle, r.x + 10, r.y + 10)
	mouse(mut h, .up, .middle, r.x + 10, r.y + 10)
	assert bench_of(h, v).tabs.map(it.id) == ['c']
}

fn test_drag_reorders_tabs() {
	mut h, v := open()!
	a := h.find('ts-a') or { panic('') }
	c := h.find('ts-c') or { panic('') }
	h.move_to(a.x + 10, a.y + 10)
	mouse(mut h, .down, .left, a.x + 10, a.y + 10)
	h.move_to(c.x + c.width / 2, c.y + 10)
	mouse(mut h, .up, .left, c.x + c.width / 2, c.y + 10)
	assert bench_of(h, v).tabs.map(it.id) == ['b', 'c', 'a']
	assert bench_of(h, v).active == 'a'
}

fn test_activity_bar_and_pane_resize() {
	mut h, v := open()!
	h.click('ab-search')!
	assert bench_of(h, v).activity == 'search'
	assert h.has_text('3') // the badge
	hd := h.find('pz-split-handle') or { panic('no divider') }
	pz := h.find('pz') or { panic('') }
	h.move_to(hd.x + 2, hd.y + 50)
	mouse(mut h, .down, .left, hd.x + 2, hd.y + 50)
	h.move_to(pz.x + pz.width * 0.25, hd.y + 50)
	mouse(mut h, .up, .left, pz.x + pz.width * 0.25, hd.y + 50)
	ratio := bench_of(h, v).root.ratio
	assert ratio > 0.2 && ratio < 0.3, 'ratio ${ratio}'
	// Double-clicking the divider resets an even split.
	h.advance(1000)
	hd2 := h.find('pz-split-handle') or { panic('') }
	h.click_at(hd2.x + 2, hd2.y + 50)
	h.click_at(hd2.x + 2, hd2.y + 50)
	assert bench_of(h, v).root.ratio == 0.5
}

// Stack: a stacked pane tree whose bottom leaf folds to its header, and an overflowing strip.
struct Stack {
mut:
	root work.PaneNode = work.vsplit(work.leaf('top'), work.leaf('bottom'), 0.5)
}

@[markused]
pub fn (mut s Stack) render(mut cx core.ViewCx[Stack]) core.Element {
	tabs := []work.Tab{len: 40, init: work.Tab{'t${index}', 'tab number ${index}', '', false}}
	fold := ui.div().test_id('fold').size(20).on_click(cx.listener(fn (mut s Stack, _ &core.ClickEvent, mut _ core.ViewCx[Stack]) {
		s.root = s.root.toggle('bottom')
	}))
	bottom :=
		ui.div().test_id('bottom').flex_col().size_full().child(ui.div().h(32).shrink_0().child(fold)).child(ui.text('bottom body'))
	panes := work.panes('st', s.root, {
		'top':    core.Element(ui.div().test_id('top').size_full().child(work.tab_strip('many',
			tabs, 't0')))
		'bottom': core.Element(bottom)
	}).header(32)
	return ui.div().size_full().child(panes)
}

fn test_leaf_folds_to_its_header() {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	h.open(Stack{}, width: 800, height: 400)!
	h.click('fold')!
	b := h.find('bottom') or { panic('') }
	top := h.find('top') or { panic('') }
	assert b.height == 32, 'folded height ${b.height}'
	assert top.height == 400 - 32 - 1
	h.click('fold')!
	b2 := h.find('bottom') or { panic('') }
	assert b2.height > 150 // open again: the split's ratio is back
}

fn test_wheel_scrolls_an_overflowing_tab_strip() {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	h.open(Stack{}, width: 800, height: 400)!
	x0 := (h.find('many-t0') or { panic('') }).x
	h.scroll_at(200, 16, -120)
	x1 := (h.find('many-t0') or { panic('') }).x
	assert x1 < x0, 'tab strip did not scroll: ${x0} -> ${x1}'
}
