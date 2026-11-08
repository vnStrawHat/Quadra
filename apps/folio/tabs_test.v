// Tabs: several documents in one window, switching, closing with and without changes, the
// untouched new document that a file replaces, and the session restored at the next start.
module main

import os
import quadra.core
import quadra.platform
import quadra.testing

struct Tabs {
mut:
	h    &testing.Harness
	view core.View[Folio]
	dir  string
}

fn (r &Tabs) f() &Folio {
	cx := core.Cx{
		app: r.h.app
	}
	return cx.peek(r.view.model())
}

fn (r &Tabs) names() []string {
	return r.f().tabs.map(it.name())
}

fn open_tabs(dir string, files []string) !Tabs {
	mut h := testing.new_harness(width: 1200, height: 800)!
	opts, f := prepare(mut h.app, os.join_path(dir, 'data'), files)!
	v := h.open(f, width: opts.width, height: opts.height)!
	return Tabs{h, v, dir}
}

fn test_tabs_open_switch_and_close() {
	dir := os.join_path(os.temp_dir(), 'quadra-tests', 'folio', 'tabs')
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	a, b := os.join_path(dir, 'a.md'), os.join_path(dir, 'b.md')
	os.write_file(a, '# A\n')!
	os.write_file(b, '# B\n')!
	mut r := open_tabs(dir, [a, b])!
	assert r.names() == ['a.md', 'b.md'] && r.f().cur().name() == 'b.md'
	r.h.press('ctrl-tab')!
	assert r.f().cur().name() == 'a.md'
	r.h.press('ctrl-pageup')!
	assert r.f().cur().name() == 'b.md'
	r.h.inject(platform.FileDropEvent{
		paths: [a]
	})
	assert r.names().len == 2 && r.f().cur().name() == 'a.md' // an open file shows its tab
	// A new document; the next file opened takes its tab while it is untouched.
	r.h.press('ctrl-n')!
	assert r.names() == ['a.md', 'Untitled.md', 'b.md']
	// Unsaved changes ask before the tab closes.
	r.h.type_text('draft')
	r.h.press('ctrl-w')!
	assert r.h.find('close-discard') != none
	r.h.click('close-discard')!
	assert r.names() == ['a.md', 'b.md'] && r.f().cur().name() == 'b.md'
	// A tab's own close button; the last one leaves the empty state.
	id := r.f().cur().id
	r.h.click('tabs-${id}-close')!
	r.h.press('ctrl-w')!
	assert r.f().tabs.len == 0 && r.h.find('empty-open') != none
}

fn test_session_comes_back() {
	dir := os.join_path(os.temp_dir(), 'quadra-tests', 'folio', 'session')
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	a, b := os.join_path(dir, 'a.md'), os.join_path(dir, 'b.md')
	os.write_file(a, '# A\n')!
	os.write_file(b, '# B\n')!
	mut r := open_tabs(dir, [a, b])!
	r.h.press('ctrl-tab')! // a.md shown
	mut cx := core.Cx{
		app: r.h.app
	}
	r.f().prefs.flush(mut cx)!
	mut again := open_tabs(dir, [])!
	assert again.names() == ['a.md', 'b.md'] && again.f().cur().name() == 'a.md'
}
