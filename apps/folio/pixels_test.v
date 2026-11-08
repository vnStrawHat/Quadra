// Folio rendered for real (WARP, DirectWrite, the bundled fonts): the welcome document in
// each layout, saved to .tmp/out/folio_*.png for review, with ink checks.
module main

import os
import quadra.platform
import quadra.testing

fn shot(mut h testing.PixelHarness, name string) ! {
	out := os.join_path(os.dir(@FILE), '..', '..', '.tmp', 'out')
	os.mkdir_all(out)!
	h.save_png(os.join_path(out, 'folio_${name}.png'))!
}

fn test_welcome_document_in_every_layout() {
	dir := os.join_path(os.temp_dir(), 'quadra-tests', 'folio', 'pixels')
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	mut h := testing.new_pixel_harness()!
	welcome := os.join_path(os.dir(@FILE), 'samples', 'welcome.md')
	opts, f := prepare(mut h.app, os.join_path(dir, 'data'), [welcome])!
	h.open(f, width: opts.width, height: opts.height)!
	shot(mut h, 'split')!
	h.click('fmt-insert')!
	shot(mut h, 'insert_menu')!
	h.press('escape')!
	h.press('ctrl-shift-p')!
	h.type_text('dark')
	shot(mut h, 'palette')!
	h.press('enter')! // Theme: One Dark Pro
	shot(mut h, 'split_dark')!
	h.press('ctrl-3')!
	shot(mut h, 'preview')!
	r := h.find('preview') or { panic('no preview') }
	for _ in 0 .. 8 {
		h.scroll_at(r.x + 200, r.y + 200, -120) // down to the math
	}
	shot(mut h, 'preview_math')!
	for _ in 0 .. 10 {
		h.scroll_at(r.x + 200, r.y + 200, -120) // the diagrams
	}
	shot(mut h, 'preview_diagrams')!
	h.press('ctrl-1')!
	shot(mut h, 'raw')!
	h.press('ctrl-3')!
	shot(mut h, 'preview_dark')!
	h.press('ctrl-2')!
	h.press('ctrl-shift-o')!
	shot(mut h, 'no_outline')!
	h.press('ctrl-shift-o')!
	readme := os.join_path(os.dir(@FILE), '..', '..', 'README.md')
	h.inject(platform.FileDropEvent{
		paths: [readme]
	})
	shot(mut h, 'tabs')!
	h.press('ctrl-w')!
	h.press('ctrl-w')!
	shot(mut h, 'empty')!
	img := h.rgba()!
	assert img.len > 0
}
