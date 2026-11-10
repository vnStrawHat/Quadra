// Links in Folio (specs/21, Links): preview links, URLs in fences and URLs in the editor open
// on ctrl-click only; a plain click opens nothing; other schemes never reach the shell.
module main

import os
import quadra.core
import quadra.testing

const link_doc = '# T\n\nSee [x](https://a.example/x).\n\n```bash\ncurl https://b.example/i.sh\n```\n\n[bad](ms-settings:privacy)\n'

fn open_links(name string) !(&testing.Harness, core.View[Folio]) {
	dir := os.join_path(os.temp_dir(), 'quadra-tests', 'folio', name)
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	path := os.join_path(dir, 'doc.md')
	os.write_file(path, link_doc)!
	mut h := testing.new_harness(width: 1200, height: 800)!
	opts, f := prepare(mut h.app, os.join_path(dir, 'data'), [path])!
	v := h.open(f, width: opts.width, height: opts.height)!
	return h, v
}

// preview_link is the middle of the k-th ctrl link hitbox in the preview (from the end when
// k < 0; a URL cut by the tokenizer has one per piece).
fn preview_link(h &testing.Harness, k int) !(f32, f32) {
	pane := h.find('preview') or { return error('no preview') }
	links := h.window().hitboxes.filter(int(it.cursor_mods) != 0
		&& pane.contains(it.bounds.center()))
	i := if k < 0 { links.len + k } else { k }
	if i < 0 || i >= links.len {
		return error('${links.len} links')
	}
	r := links[i].bounds
	return r.x + r.width / 2, r.y + r.height / 2
}

fn test_preview_links_open_on_ctrl_click_only() {
	mut h, _ := open_links('links-preview')!
	x, y := preview_link(h, 0)!
	h.click_at(x, y)
	assert h.plat.opened.len == 0
	h.click_mods(x, y, .ctrl)
	assert h.plat.opened == ['https://a.example/x']
	// The URL in the bash fence.
	fx, fy := preview_link(h, 1)!
	h.click_at(fx, fy)
	assert h.plat.opened.len == 1
	h.click_mods(fx, fy, .ctrl)
	assert h.plat.opened.last() == 'https://b.example/i.sh'
	// Another scheme opens nothing and is not taken for a file.
	bx, by := preview_link(h, -1)!
	h.click_mods(bx, by, .ctrl)
	assert h.plat.opened.len == 2 && !h.has_text('Not found: ms-settings:privacy')
}

fn test_editor_urls_follow_the_preview_rules() {
	mut h, v := open_links('links-editor')!
	mut cx := core.Cx{
		app: h.app
	}
	f := cx.peek(v.model())
	e := cx.peek(f.cur().editor.model())
	head := e.doc.primary().head
	x := e.bounds.x + e.cell.width * 12.5 // inside `https://a.example/x` on line 2
	y := e.bounds.y + e.cell.height * 2.5 - e.scroll.y
	h.click_mods(x, y, .ctrl)
	assert h.plat.opened == ['https://a.example/x']
	assert e.doc.primary().head == head && e.doc.cursors.len == 1
	h.click_at(x, y)
	assert h.plat.opened.len == 1 && e.doc.primary().head > head
}
