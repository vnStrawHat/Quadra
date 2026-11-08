module main

import os
import quadra.testing

fn test_counter_renders_real_pixels() {
	mut h := testing.new_pixel_harness()!
	h.open(Counter{}, width: 480, height: 320, scale: 1.5)!
	h.click('inc')!
	out := os.join_path(os.dir(@FILE), '..', '..', '.tmp', 'out')
	os.mkdir_all(out)!
	h.save_png(os.join_path(out, 'counter_150.png'))!
	px := h.rgba()!
	assert px.len == 720 * 480 * 4
}
