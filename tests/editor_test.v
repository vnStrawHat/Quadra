// The code editor view in the harness (specs/18, Tests): typing, clicks, selection,
// multiple cursors, find and replace, the headless clipboard, IME and scrolling.
module main

import math
import quadra.core
import quadra.core.life
import quadra.kit
import quadra.kit.code
import quadra.platform
import quadra.testing
import quadra.ui

struct Host {
mut:
	ed      core.View[code.Editor]
	text    string
	path    string = 'main.v'
	wrap    bool
	changes int
}

@[markused]
pub fn (mut h Host) render(mut cx core.ViewCx[Host]) core.Element {
	if h.ed.id == 0 {
		h.ed = code.new_editor(mut cx.Cx, id: 'ed', text: h.text, path: h.path, wrap: h.wrap)
		code.on_change(mut cx.Cx, h.ed, cx.listener(fn (mut h Host, _ &code.Doc, mut cx core.ViewCx[Host]) {
			h.changes++
		}))
	}
	return ui.div().size_full().child(cx.child_view(h.ed))
}

fn open(text string) !(&testing.Harness, &Host, &code.Editor) {
	return open_with(text, false)
}

fn open_with(text string, wrap bool) !(&testing.Harness, &Host, &code.Editor) {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(Host{
		text: text
		wrap: wrap
	},
		width:  800
		height: 400
	)!
	mut cx := core.Cx{
		app: h.app
	}
	host := cx.peek(v.model())
	return h, host, cx.peek(host.ed.model())
}

// at is the window point of line n, column col.
fn at(e &code.Editor, n int, col int) (f32, f32) {
	return e.bounds.x + f32(col) * e.cell.width + 1, e.bounds.y + f32(n) * e.cell.height -
		e.scroll.y + e.cell.height / 2
}

fn click(mut h testing.Harness, e &code.Editor, n int, col int, mods platform.Mods) {
	x, y := at(e, n, col)
	h.move_to(x, y)
	h.inject(platform.MouseEvent{ action: .down, button: .left, x: x, y: y, mods: mods })
	h.inject(platform.MouseEvent{ action: .up, button: .left, x: x, y: y, mods: mods })
}

fn test_typing_indent_and_undo() {
	mut h, host, e := open('')!
	h.click('ed')!
	h.type_text('fn main() {')
	h.press('enter')!
	h.type_text('x := 1')
	assert e.value() == 'fn main() {\n    x := 1'
	assert host.changes > 0
	h.press('ctrl-z')!
	assert e.value() == 'fn main() {\n    '
	h.press('ctrl-z')!
	h.press('ctrl-z')!
	assert e.value() == ''
	h.press('ctrl-y')!
	assert e.value() == 'fn main() {'
	h.press('tab')! // one indent, not two (the tab key and its character)
	assert e.value() == 'fn main() { ' // to the next tab stop
	assert h.has_text('fn main() { ')
}

fn test_clicks_and_selection() {
	mut h, _, e := open('alpha beta\ngamma delta\nlast')!
	h.frame()
	click(mut h, e, 1, 3, platform.Mods.zero())
	assert e.doc.primary().head == 14 // line 1 starts at 11
	click(mut h, e, 0, 2, platform.Mods.shift)
	assert e.doc.selected_texts() == ['pha beta\ngam'] // from the click back to column 2
	// A double click selects the word, a triple click the line.
	click(mut h, e, 1, 8, platform.Mods.zero())
	click(mut h, e, 1, 8, platform.Mods.zero())
	assert e.doc.selected_texts() == ['delta']
	click(mut h, e, 1, 8, platform.Mods.zero())
	assert e.doc.selected_texts() == ['gamma delta\n']
	h.advance(1000)
	// Dragging selects.
	x0, y0 := at(e, 0, 0)
	x1, y1 := at(e, 0, 5)
	h.move_to(x0, y0)
	h.inject(platform.MouseEvent{ action: .down, button: .left, x: x0, y: y0 })
	h.move_to(x1, y1)
	h.inject(platform.MouseEvent{ action: .up, button: .left, x: x1, y: y1 })
	assert e.doc.selected_texts() == ['alpha']
	// ctrl-click adds a cursor; typing goes to both.
	h.advance(1000)
	click(mut h, e, 2, 0, platform.Mods.zero())
	click(mut h, e, 0, 0, platform.Mods.ctrl)
	h.type_text('>')
	assert e.value() == '>alpha beta\ngamma delta\n>last'
	// alt-drag makes a column of cursors.
	h.advance(1000)
	xa, ya := at(e, 0, 1)
	xb, yb := at(e, 2, 1)
	h.move_to(xa, ya)
	h.inject(platform.MouseEvent{ action: .down, button: .left, x: xa, y: ya, mods: .alt })
	h.move_to(xb, yb)
	h.inject(platform.MouseEvent{ action: .up, button: .left, x: xb, y: yb, mods: .alt })
	assert e.doc.cursors.len == 3
}

fn test_keys_for_lines_and_cursors() {
	mut h, _, e := open('one\ntwo\nthree')!
	h.click('ed')!
	h.press('ctrl-home')!
	h.press('alt-down')!
	assert e.value() == 'two\none\nthree'
	h.press('ctrl-shift-d')!
	assert e.value() == 'two\none\none\nthree'
	h.press('ctrl-shift-k')!
	h.press('ctrl-/')!
	assert e.value().starts_with('two\none\n// three') || e.value().contains('// ')
	h.press('ctrl-z')!
	h.press('ctrl-home')!
	h.press('ctrl-alt-down')!
	h.press('ctrl-alt-down')!
	assert e.doc.cursors.len == 3
	h.press('escape')!
	assert e.doc.cursors.len == 1
	h.press('ctrl-a')!
	h.press('ctrl-c')!
	assert h.plat.clipboard == e.value()
	h.press('end')!
	h.plat.clipboard = '\nend'
	h.press('ctrl-v')!
	assert e.value().ends_with('\nend')
}

fn test_find_and_replace_bar() {
	mut h, _, e := open('foo bar foo\nbaz foo')!
	h.click('ed')!
	h.press('ctrl-f')!
	assert h.find('ed-findbar') != none
	h.type_text('foo') // the query input has focus
	assert e.doc.selected_texts() == ['foo']
	assert h.has_text('1 of 3')
	h.press('enter')!
	assert h.has_text('2 of 3')
	h.press('f3')!
	assert h.has_text('3 of 3')
	h.press('escape')!
	assert h.find('ed-findbar') == none
	assert e.value() == 'foo bar foo\nbaz foo' // typing went to the query, not the text
	h.press('ctrl-h')!
	h.click('ed-replace')!
	h.type_text('x')
	h.click('ed-replace-all')!
	assert e.value() == 'x bar x\nbaz x'
	h.click('ed')!
	h.press('ctrl-z')!
	assert e.value() == 'foo bar foo\nbaz foo' // replace all is one undo step
}

fn test_ime_composes_inline() {
	mut h, _, e := open('')!
	h.click('ed')!
	h.inject(platform.ImeEvent{ kind: .start })
	h.inject(platform.ImeEvent{ kind: .preedit, text: 'ni', cursor: 2 })
	assert e.preedit == 'ni' && e.value() == ''
	h.inject(platform.ImeEvent{ kind: .commit, text: '你' })
	h.inject(platform.ImeEvent{ kind: .end })
	assert e.value() == '你' && e.preedit == ''
}

fn test_scrolling_and_reveal() {
	mut lines := []string{len: 2000, init: 'line ${index}'}
	mut h, _, e := open(lines.join('\n'))!
	h.frame()
	assert h.has_text('line 0') && !h.has_text('line 500')
	r := h.find('ed') or { panic('no editor') }
	h.scroll_at(r.x + 100, r.y + 100, -600)
	assert e.scroll.y > 0
	assert !h.has_text('line 0')
	h.click('ed')!
	h.press('ctrl-end')!
	assert h.has_text('line 1999') // the cursor is kept visible
	h.press('ctrl-home')!
	assert h.has_text('line 0')
}

fn test_repaints_do_not_leak_closures() {
	mut h, _, _ := open('fn main() {\n\tprintln("hi")\n}')!
	h.click('ed')!
	for _ in 0 .. 5 {
		h.advance(600) // the caret blinks
		h.type_text('a')
	}
	before := life.live_closures()
	for _ in 0 .. 20 {
		h.advance(600)
		h.type_text('b')
		h.press('left')!
	}
	assert life.live_closures() == before
}

fn test_find_follows_a_new_document() {
	mut h, host, _ := open('foo foo')!
	h.click('ed')!
	h.press('ctrl-f')!
	h.type_text('foo')
	assert h.has_text('1 of 2')
	h.press('shift-enter')! // back, wrapping
	assert h.has_text('2 of 2')
	// A new document starts at version 0 too; the old matches must not survive.
	mut cx := core.Cx{
		app: h.app
	}
	code.set_text(mut cx, host.ed, 'foo')
	h.frame()
	assert h.has_text('1 found')
}

fn test_soft_wrap_rows() {
	long := 'x'.repeat(300)
	mut h, _, mut e := open_with([]string{len: 30, init: long}.join('
') + '
next', true)!
	h.frame()
	cols := math.max(8, int(e.bounds.width / e.cell.width) - 1)
	rows := (300 + cols - 1) / cols // a word longer than a row breaks inside it
	assert rows > 1
	click(mut h, e, 1, 2, platform.Mods.zero()) // the second row of line 0
	assert e.doc.primary().head == cols + 2
	h.press('down')! // the next visual row, same column
	assert e.doc.primary().head == 2 * cols + 2
	h.press('ctrl-end')!
	assert h.has_text('next') && e.scroll.y > 0 && e.scroll.x == 0
	e.scroll_to_line(10)
	assert e.scroll.y == f32(10 * rows) * e.cell.height
	assert e.top_line() == 10
	e.scroll.y += e.cell.height * f32(rows) / 2
	assert math.abs(e.top_line() - 10.5) < 0.01
}
