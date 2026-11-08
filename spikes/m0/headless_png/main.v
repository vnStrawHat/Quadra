// M0 headless proof, on the real framework: renders a small view with WARP and
// DirectWrite through the pixel harness and writes a PNG. No window, no desktop,
// no temp files. Usage: headless_png <absolute-output.png> [scale]
module main

import os
import quadra.core
import quadra.prim
import quadra.platform.win
import quadra.testing
import quadra.ui

struct Card {}

@[markused]
pub fn (mut c Card) render(mut cx core.ViewCx[Card]) core.Element {
	return ui.div().size_full().p(40).bg(prim.rgb(0xF3F4F6)).child(ui.div().flex_col().gap(8).p(24).bg(prim.white).rounded(12).border(1).border_color(prim.rgb(0xD0D5DD)).shadow_level(2).child(ui.text('Xin chào Việt Nam').size(22).weight(600).color(prim.rgb(0x111827))).child(ui.text('Tiếng Việt có dấu: ắ ằ ẳ ẵ ặ ơ ư đ — rendered headless.').color(prim.rgb(0x4B5563))))
}

fn run(out string, scale f32) ! {
	mut h := testing.new_pixel_harness()!
	h.open(Card{}, width: 700, height: 300, scale: scale)!
	h.save_png(out)!
}

fn main() {
	if os.args.len < 2 {
		eprintln('usage: headless_png <absolute-output.png> [scale]')
		exit(2)
	}
	out := os.args[1]
	scale := if os.args.len > 2 { os.args[2].f32() } else { f32(1) }
	mut log := [
		'user ${os.getenv('USERNAME')} session ${win.session_id()} scale ${scale}',
	]
	run(out, scale) or {
		log << 'FAIL ${err}'
		os.write_file(out + '.log', log.join('\n') + '\n') or {}
		eprintln(log.join('\n'))
		exit(1)
	}
	log << 'PASS wrote ${out}'
	os.write_file(out + '.log', log.join('\n') + '\n') or {}
	println(log.join('\n'))
}
