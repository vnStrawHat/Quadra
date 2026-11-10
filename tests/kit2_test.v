// M4 components: select, date picker, number input, combobox, palette, forms, table, tree.
module main

import time
import quadra.core
import quadra.kit
import quadra.kit.data
import quadra.kit.input
import quadra.platform
import quadra.testing
import quadra.ui

struct Forms {
mut:
	fruit  int      = -1
	date   kit.Date = kit.Date{2026, 11, 7}
	num    core.View[input.NumberInput]
	amount f64
	combo  core.View[input.Combobox]
	city   int = -1
	answer string
	ran    string
}

@[markused]
pub fn (mut f Forms) render(mut cx core.ViewCx[Forms]) core.Element {
	if f.num.id == 0 {
		f.num = input.new_number_input(mut cx.Cx, id: 'qty', value: 5, step: 2, min: 0, max: 9)
		input.on_number_change(mut cx.Cx, f.num, cx.listener(fn (mut f Forms, v &f64, mut _ core.ViewCx[Forms]) {
			f.amount = *v
		}))
		f.combo = input.new_combobox(mut cx.Cx, 'city', ['Hà Nội', 'Hải Phòng', 'Đà Nẵng',
			'Huế'], 'Thành phố')
		input.on_pick(mut cx.Cx, f.combo, cx.listener(fn (mut f Forms, i &int, mut _ core.ViewCx[Forms]) {
			f.city = *i
		}))
	}
	return ui.div().flex_col().gap(8).p(8).child(kit.select_box('fruit', ['Táo', 'Cam', 'Xoài'],
		f.fruit).on_change(cx.listener(fn (mut f Forms, i &int, mut _ core.ViewCx[Forms]) {
		f.fruit = *i
	}))).child(kit.date_picker('day', f.date).on_change(cx.listener(fn (mut f Forms, d &kit.Date, mut _ core.ViewCx[Forms]) {
		f.date = *d
	}))).child(cx.child_view(f.num)).child(cx.child_view(f.combo)).child(kit.button('ask').label('Ask').on_click(cx.listener(fn (mut f Forms, _ &core.ClickEvent, mut cx core.ViewCx[Forms]) {
		kit.confirm(mut cx.Cx, 'Xoá?', 'Xoá mục này?', 'Xoá', cx.listener(fn (mut f Forms, yes &bool, mut _ core.ViewCx[Forms]) {
			f.answer = if *yes { 'yes' } else { 'no' }
		}))
	})))
}

fn forms(h &testing.Harness, v core.View[Forms]) &Forms {
	cx := core.Cx{
		app: h.app
	}
	return cx.peek(v.model())
}

fn setup_forms() (&testing.Harness, core.View[Forms]) {
	mut h := testing.new_harness() or { panic(err) }
	kit.install(mut h.app) or { panic(err) }
	v := h.open(Forms{}) or { panic(err) }
	return h, v
}

fn test_select_and_date_picker() {
	mut h, v := setup_forms()
	h.click('fruit')!
	h.click('menu-2')!
	assert forms(h, v).fruit == 2
	h.click('day')!
	h.click('cal-next')! // December
	h.click('day-24')!
	assert forms(h, v).date == kit.Date{2026, 12, 24}
}

fn test_number_input_steps_and_clamps() {
	mut h, v := setup_forms()
	h.click('num-up')!
	assert forms(h, v).amount == 7
	h.click('qty')!
	h.press('up')!
	assert forms(h, v).amount == 9 // clamped to max
	h.press('ctrl-a')!
	h.type_text('3a')
	assert forms(h, v).amount == 3 // letters are filtered out
}

fn test_combobox_filters_and_picks() {
	mut h, v := setup_forms()
	h.click('city')!
	h.type_text('hai')
	assert h.find('opt-1') != none // Hải Phòng matches "hai" (Hà Nội does not)
	h.press('enter')!
	assert forms(h, v).city == 1
}

fn test_confirm_dialog() {
	mut h, v := setup_forms()
	h.click('ask')!
	h.click('confirm-yes')!
	assert forms(h, v).answer == 'yes'
	h.click('ask')!
	h.press('escape')! // dismissed: no answer
	h.click('ask')!
	h.click('confirm-no')!
	assert forms(h, v).answer == 'no'
	// card dialogs (Controls.card_dialogs): a confirm is an alert, an outside click keeps it
	mut t := kit.dark()
	t.ctl.card_dialogs = true
	kit.set_theme(mut h.app, t)
	h.click('ask')!
	assert h.find('confirm-yes') != none && h.find('modal-close') == none
	h.click_at(2, 2)
	h.frame()
	assert h.find('confirm-yes') != none
	h.click('confirm-yes')!
	assert forms(h, v).answer == 'yes'
}

fn test_validation_rules() {
	rules := [kit.required('Bắt buộc'), kit.min_len(3, 'Quá ngắn'),
		kit.email('Email sai')]
	assert kit.validate('', rules) or { '' } == 'Bắt buộc'
	assert kit.validate('ab', rules) or { '' } == 'Quá ngắn'
	assert kit.validate('abc', rules) or { '' } == 'Email sai'
	assert kit.validate('an@quadra.dev', rules) == none
}

struct Palette {
mut:
	ran string
}

@[markused]
pub fn (mut p Palette) render(mut cx core.ViewCx[Palette]) core.Element {
	return ui.div().size_full()
}

fn test_command_palette_runs_the_best_match() {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(Palette{})!
	mut cx := core.Cx{
		app: h.app
		win: h.window()
	}
	pid := v.id
	input.open_command_palette(mut cx, [
		input.Command{'Mở tệp', 'ctrl-o', fn [pid] (mut cx core.Cx) {
			mut p := unsafe { &Palette(cx.app.store_ptr(pid)) }
			p.ran = 'open'
		}},
		input.Command{'Lưu tệp', 'ctrl-s', fn [pid] (mut cx core.Cx) {
			mut p := unsafe { &Palette(cx.app.store_ptr(pid)) }
			p.ran = 'save'
		}},
	])
	h.frame()
	h.type_text('lưu')
	h.press('enter')!
	assert cx.peek(v.model()).ran == 'save'
}

struct Grid {
mut:
	table core.View[data.Table]
	tree  core.View[data.Tree]
	sel   []int
	node  string
}

@[markused]
pub fn (mut g Grid) render(mut cx core.ViewCx[Grid]) core.Element {
	if g.table.id == 0 {
		g.table = data.new_table(mut cx.Cx,
			id:      'tbl'
			columns: [data.Column{
				title: 'Name'
			}, data.Column{
				title:   'Size'
				numeric: true
			}]
			rows:    1_000_000
			cell:    fn (row int, col int) string {
				return if col == 0 { 'file ${row:07}' } else { '${(i64(row) * 7919) % 1000}' }
			}
		)
		data.on_select(mut cx.Cx, g.table, cx.listener(fn (mut g Grid, s &[]int, mut _ core.ViewCx[Grid]) {
			g.sel = *s
		}))
		g.tree = data.new_tree(mut cx.Cx, 'tree', [
			data.TreeNode{'src', 'src', '', [
				data.TreeNode{'core', 'core', '', []},
				data.TreeNode{'ui', 'ui', '', []},
			]},
			data.TreeNode{'docs', 'docs', '', []},
		])
		data.on_tree_select(mut cx.Cx, g.tree, cx.listener(fn (mut g Grid, id &string, mut _ core.ViewCx[Grid]) {
			g.node = *id
		}))
	}
	return ui.div().flex_row().size_full().child(ui.div().w(500).h_full().child(cx.child_view(g.table))).child(ui.div().w(200).h_full().child(cx.child_view(g.tree)))
}

fn test_table_virtualizes_selects_and_sorts() {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(Grid{})!
	cx := core.Cx{
		app: h.app
	}
	assert h.has_text('file 0000000')
	assert h.window().stats.instances < 2000 // a million rows, one screenful drawn
	h.click('tbl-row3')!
	h.inject(platform.MouseEvent{
		action: .down
		button: .left
		x:      50
		y:      (h.find('tbl-row5') or { panic('') }).y + 5
		mods:   .shift
	})
	h.inject(platform.MouseEvent{
		action: .up
		button: .left
		x:      50
		y:      (h.find('tbl-row5') or { panic('') }).y + 5
		mods:   .shift
	})
	assert cx.peek(v.model()).sel == [3, 4, 5]
	h.press('down')!
	assert cx.peek(v.model()).sel == [6]
	sw := time.new_stopwatch()
	h.click('tbl-h1')! // sort a million rows by size
	assert sw.elapsed().seconds() < 10
	assert h.has_text('0') // smallest size first
	r := h.find('tbl-r0') or { panic('') }
	h.move_to(r.x + 2, r.y + 5)
	h.inject(platform.MouseEvent{
		action: .down
		button: .left
		x:      r.x + 2
		y:      r.y + 5
	})
	h.inject(platform.MouseEvent{
		action: .move
		x:      r.x + 62
		y:      r.y + 5
	})
	h.inject(platform.MouseEvent{
		action: .up
		button: .left
		x:      r.x + 62
		y:      r.y + 5
	})
	h0 := h.find('tbl-h0') or { panic('') }
	assert h0.width > 190 // 140 + 60, minus the handle
	// Headers line up with the data columns: column 1 starts exactly one column width later.
	assert (h.find('tbl-h1') or { panic('') }).x == h0.x + h0.width + 5
}

fn test_tree_keyboard_navigation() {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(Grid{})!
	cx := core.Cx{
		app: h.app
	}
	h.click('tree-src')!
	assert cx.peek(v.model()).node == 'src'
	assert h.find('tree-core') == none // collapsed
	h.press('right')!
	assert h.find('tree-core') != none
	h.press('down')!
	assert cx.peek(v.model()).node == 'core'
	h.press('left')! // to the parent
	assert cx.peek(v.model()).node == 'src'
}
