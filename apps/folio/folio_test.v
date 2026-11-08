// Folio flows in the harness: open, the three layouts, live edit, outline, save, the close
// prompt, outside changes, the toolbar. Files live under .tmp (tools/v.sh points the temp
// folder there). Tabs and the session are in tabs_test.v.
module main

import os
import quadra.core
import quadra.testing

struct Run {
mut:
	h    &testing.Harness
	view core.View[Folio]
	dir  string
}

fn start(name string, text string) !Run {
	dir := os.join_path(os.temp_dir(), 'quadra-tests', 'folio', name)
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	path := os.join_path(dir, 'doc.md')
	os.write_file(path, text)!
	mut h := testing.new_harness(width: 1200, height: 800)!
	opts, f := prepare(mut h.app, os.join_path(dir, 'data'), [path])!
	v := h.open(f, width: opts.width, height: opts.height)!
	return Run{h, v, dir}
}

fn (r &Run) f() &Folio {
	cx := core.Cx{
		app: r.h.app
	}
	return cx.peek(r.view.model())
}

// t is the shown tab.
fn (r &Run) t() &Tab {
	return r.f().cur()
}

const doc = '# Title\n\nSome **text**.\n\n## Second\n\nMore.\n\n## Third\n\nEnd.\n'

fn test_opens_and_switches_layouts() {
	mut r := start('layouts', doc)!
	assert r.f().tabs.len == 1 && r.t().text == doc
	assert r.h.has_text('doc.md')
	assert r.h.find('editor-pane') != none && r.h.find('preview') != none // Split
	r.h.press('ctrl-1')!
	assert r.h.find('preview') == none && r.h.find('editor-pane') != none
	r.h.press('ctrl-3')!
	assert r.h.find('editor-pane') == none && r.h.find('preview') != none
	r.h.click('fmt-split')! // the layout buttons on the toolbar
	assert r.h.find('editor-pane') != none && r.h.find('preview') != none
	r.h.press('ctrl-shift-o')!
	assert r.h.find('outline') == none
	r.h.press('ctrl-shift-o')!
	assert r.h.find('outline-1') != none
	assert r.h.has_text('1:1')
}

fn test_live_edit_dirty_and_save() {
	mut r := start('edit', doc)!
	r.h.click('editor-pane')!
	r.h.press('ctrl-end')!
	r.h.type_text('\n## Added heading\n')
	assert r.h.has_text('Added heading') // the preview and the outline have it at once
	assert r.t().dirty() && r.f().dirty()
	r.h.press('ctrl-s')!
	assert !r.t().dirty()
	assert os.read_file(os.join_path(r.dir, 'doc.md'))!.contains('## Added heading')
}

fn test_outline_click_moves_the_editor() {
	mut long := doc
	for i in 0 .. 80 {
		long += 'line ${i}\n\n'
	}
	long += '## Far down\n\nbottom\n'
	for i in 0 .. 80 {
		long += 'after ${i}\n\n' // room to scroll the heading to the top
	}
	mut r := start('outline', long)!
	r.h.click('outline-3')!
	cx := core.Cx{
		app: r.h.app
	}
	ed := cx.peek(r.t().editor.model())
	line := ed.doc.buf.line_of(ed.doc.primary().head)
	assert ed.doc.buf.line(line) == '## Far down'
	assert ed.scroll.y > 0
	assert r.t().scroll.y > 0 // the preview went there too
	r.h.frame()
	assert r.t().sync.section == 3 // and the outline marks the section
	assert r.h.has_text('Far down') // in the breadcrumb too
	// The search field filters the outline.
	r.h.click('outline-filter')!
	r.h.type_text('thi')
	assert r.h.find('outline-2') != none && r.h.find('outline-1') == none
}

fn test_close_prompt_and_outside_changes() {
	mut r := start('close', doc)!
	r.h.click('editor-pane')!
	r.h.type_text('x')
	r.h.press('ctrl-shift-w')!
	assert r.h.find('close-cancel') != none // asked, still open
	r.h.click('close-cancel')!
	assert r.h.app.windows.len == 1
	// Changed on disk with unsaved changes here: the banner asks.
	os.write_file(os.join_path(r.dir, 'doc.md'), '# Other\n')!
	mut t := r.t()
	t.mtime = 0
	r.h.advance(2100)
	assert r.h.find('outside-banner') != none
	r.h.click('outside-reload')!
	assert r.t().text == '# Other\n' && !r.t().dirty()
}

fn test_toolbar_formats_and_keeps_the_editor_focused() {
	mut r := start('toolbar', 'hello\n')!
	r.h.click('editor-pane')!
	r.h.press('ctrl-a')!
	r.h.click('fmt-bold')!
	assert r.t().text == '**hello**\n'
	assert r.h.has_text('hello') // the preview has it at once
	r.h.type_text('x') // typing goes back to the editor, over the selection
	assert r.t().text == '**x**\n'
	r.h.press('ctrl-shift-8')!
	assert r.t().text == '- **x**\n'
	r.h.press('ctrl-z')!
	assert r.t().text == '**x**\n'
	r.h.press('ctrl-end')!
	r.h.click('fmt-insert')!
	r.h.click('menu-4')! // Table
	assert r.t().text.contains('| Column 1 | Column 2 |')
	r.h.type_text('Name')
	assert r.t().text.contains('| Name | Column 2 |')
	r.h.click('fmt-heading')!
	r.h.click('menu-1')! // Heading 1, on the table's first line
	assert r.t().text.contains('# | Name')
	r.h.press('ctrl-alt-2')!
	r.h.press('ctrl-shift-q')!
	assert r.t().text.contains('> ## | Name')
	r.h.press('ctrl-3')!
	assert r.h.find('fmt-bold') == none // Preview has no editor, no formatting
}
