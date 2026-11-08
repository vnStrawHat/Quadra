// Renders every gallery page headlessly (WARP + DirectWrite) to PNGs for design review.
// Pages with popups are shot a second time with the popup open (<id>-open.png).
//   bench/shots [out_dir] [light|dark] [page ids...]   (SHOT_SCALE=1.5 for HiDPI shots)
module main

import os
import apps.gallery.pages
import quadra.kit
import quadra.platform
import quadra.testing

// openers maps a page to the element that opens its popup.
const openers = {
	'menu':         'mn-file'
	'select':       'sel-ns'
	'date-picker':  'dp-date'
	'dialog':       'dl-open'
	'confirm':      'cf-open'
	'toast':        'to-info'
	'palette':      'pal-open'
	'tooltip':      'tt-btn'
	'context-menu': ''
}

fn main() {
	out := if os.args.len > 1 { os.args[1] } else { '.tmp/shots' }
	dark := os.args.len > 2 && os.args[2] == 'dark'
	only := if os.args.len > 3 { os.args[3..] } else { []string{} }
	os.mkdir_all(out)!
	mut ph := testing.new_pixel_harness()!
	mut h := &ph.Harness
	kit.install(mut h.app)!
	kit.set_theme(mut h.app, if dark { kit.dark() } else { kit.light() })
	scale := f32(os.getenv_opt('SHOT_SCALE') or { '1' }.f64())
	h.open(pages.Gallery{}, width: 1200, height: 800, scale: scale)!
	for p in pages.all() {
		if only.len > 0 && p.id !in only {
			continue
		}
		for k in 0 .. 16 {
			if h.find('nav-${p.id}') != none {
				break
			}
			h.scroll_at(100, 400, if k < 8 { -400 } else { 400 })
		}
		h.click('nav-${p.id}') or { continue }
		h.wait_images() or {}
		h.advance(300)
		ph.save_png('${out}/${p.id}.png')!
		println('${out}/${p.id}.png')
		if p.id !in openers {
			continue
		}
		target := openers[p.id]
		if target == '' {
			h.move_to(450, 260)
			for a in [platform.MouseAction.down, .up] {
				h.inject(platform.MouseEvent{ action: a, button: .right, x: 450, y: 260 })
			}
		} else if p.id == 'tooltip' {
			r := h.find(target) or { continue }
			h.move_to(r.x + 10, r.y + 10)
			h.advance(700)
		} else {
			h.click(target) or { continue }
		}
		if p.id == 'toast' {
			h.click('to-err') or {}
		}
		h.advance(300)
		ph.save_png('${out}/${p.id}-open.png')!
		println('${out}/${p.id}-open.png')
		h.press('escape') or {}
		h.advance(4500) // toasts expire
	}
}
