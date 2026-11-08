module main

import os
import apps.gallery.pages
import quadra.kit
import quadra.core.life
import quadra.platform
import quadra.testing

fn test_every_page_renders() {
	mut h := testing.new_harness(width: 1200, height: 800)!
	kit.install(mut h.app)!
	h.open(pages.Gallery{}, width: 1200, height: 800)!
	for p in pages.all() {
		open_page(mut h, p.id)!
		heading := if p.id == 'start' { 'Quadra' } else { p.title }
		assert h.has_text(heading), 'page ${p.id} did not render its heading'
	}
}

fn test_gallery_pixels() {
	out := os.join_path(os.dir(@FILE), '..', '..', '.tmp', 'out')
	os.mkdir_all(out)!
	mut h := testing.new_pixel_harness()!
	kit.install(mut h.app)!
	install_commands(mut h.app)!
	h.open(pages.Gallery{}, width: 1200, height: 800)!
	for id in ['button', 'checkbox', 'tabs', 'menu', 'text-input', 'field', 'table', 'editor',
		'rich-text', 'variable-list', 'markdown', 'alert', 'empty-state', 'dock'] {
		open_page(mut h.Harness, id)!
		h.save_png(os.join_path(out, 'gallery_${id}.png'))!
	}
	tab := h.find('dock-left-main') or { panic('no tab') } // the last page above is the dock
	h.drag(tab.x + 20, tab.y + 10, 1120, 480, false) // mid-drag: preview chip, right-edge zone
	h.save_png(os.join_path(out, 'gallery_dock_drag.png'))!
	h.press('escape')!
	h.inject(platform.MouseEvent{ action: .up, button: .left, x: 1120, y: 480 })
	open_page(mut h.Harness, 'motion')!
	card := h.find('mo-grow') or { panic('no card') }
	h.move_to(card.x + 20, card.y + 20)
	h.advance(400) // the transition settles
	h.frame()
	h.save_png(os.join_path(out, 'gallery_motion_hover.png'))!
	turn := h.find('mo-turn') or { panic('no turn card') }
	h.move_to(turn.x + 20, turn.y + 20)
	h.advance(400)
	h.frame()
	h.save_png(os.join_path(out, 'gallery_motion_turn.png'))!
	open_page(mut h.Harness, 'drawer')!
	h.click('drawer-right')!
	h.save_png(os.join_path(out, 'gallery_drawer.png'))!
	h.press('escape')!
	h.press('ctrl-k')!
	h.type_text('giao dien') // no diacritics: still finds 'Đổi giao diện sáng/tối'
	assert h.has_text('Đổi giao diện sáng/tối')
	h.save_png(os.join_path(out, 'gallery_palette.png'))!
	h.press('escape')!
	h.click('theme-toggle')!
	open_page(mut h.Harness, 'status')!
	h.save_png(os.join_path(out, 'gallery_status_dark.png'))!
	open_page(mut h.Harness, 'editor')!
	h.click('demo-editor')!
	h.press('ctrl-f')!
	h.type_text('point')
	h.save_png(os.join_path(out, 'gallery_editor_dark.png'))!
}

// open_page opens a page, scrolling the sidebar like a user would when it is below the fold.
fn open_page(mut h testing.Harness, id string) ! {
	for k in 0 .. 16 { // down first, then back up
		if r := h.find('nav-${id}') {
			if r.y >= 36 && r.y + r.height / 2 <= 770 { // its center clear of the status bar
				break
			}
		}
		h.scroll_at(100, 400, if k < 8 { -400 } else { 400 })
	}
	h.click('nav-${id}')!
}

// Closures made while painting, laying out or hovering have no owner and are never freed
// (specs/14); revisiting pages and showing tooltips must not add any.
fn test_no_closures_leak_on_revisits_and_tooltips() {
	mut h := testing.new_harness(width: 1200, height: 800)!
	kit.install(mut h.app)!
	h.open(pages.Gallery{}, width: 1200, height: 800)!
	ids := ['spinner', 'split', 'panes', 'icon-button', 'activity-bar', 'terminal', 'editor',
		'live-table', 'log-view', 'value-viewer', 'line-chart']
	round := fn [ids] (mut h testing.Harness) ! {
		for id in ids {
			open_page(mut h, id)!
			for y := f32(100); y < 700; y += 40 {
				for x := f32(260); x < 1200; x += 60 {
					h.move_to(x, y)
				}
			}
			h.advance(600) // tooltip delay, spinner frames
		}
	}
	round(mut h)!
	round(mut h)!
	before := life.live_closures()
	round(mut h)!
	assert life.live_closures() == before
}

// Docking end to end: a torn-off tab dropped on a pane's right edge splits it; dropped on
// another strip it moves there.
fn test_dock_page_splits_and_moves_tabs() {
	mut h := testing.new_harness(width: 1200, height: 800)!
	kit.install(mut h.app)!
	h.open(pages.Gallery{}, width: 1200, height: 800)!
	open_page(mut h, 'dock')!
	tab := h.find('dock-left-main') or { panic('no tab') }
	pane := h.find('dock-right') or { panic('no strip') }
	x0, y0 := tab.x + tab.width / 2, tab.y + tab.height / 2
	h.drag(x0, y0, x0, y0 + 30, false)
	h.drag(x0, y0 + 30, pane.x + pane.width - 10, pane.y + 140, true)
	assert h.find('dock-group-1') != none, 'the right edge should split off a new group'
	assert h.find('dock-group-1-main') != none
	strip := h.find('dock-right') or { panic('no strip') }
	app_tab := h.find('dock-left-app') or { panic('no app tab') }
	ax, ay := app_tab.x + 10, app_tab.y + 10
	h.drag(ax, ay, ax, ay + 40, false) // tear it off first: sideways it would reorder
	tx, ty := strip.x + strip.width - 4, strip.y + strip.height / 2
	h.move_to(tx, ty)
	h.inject(platform.MouseEvent{ action: .up, button: .left, x: tx, y: ty })
	assert h.find('dock-right-app') != none, 'a drop on a strip moves the tab there'
	assert h.find('dock-left') == none, 'the emptied group closes'
}

// A drawer is a modal against an edge: full height on the right, escape closes it.
fn test_drawer_opens_at_the_edge_and_closes() {
	mut h := testing.new_harness(width: 1200, height: 800)!
	kit.install(mut h.app)!
	install_commands(mut h.app)!
	h.open(pages.Gallery{}, width: 1200, height: 800)!
	open_page(mut h, 'drawer')!
	h.click('drawer-right')!
	r := h.find_role(.dialog, 'Pod api-7d9f8c') or { panic('no drawer') }
	assert r.bounds.right() == 1200
	assert r.bounds.height == 800
	assert r.bounds.width == 360
	assert h.has_text('pool-a-3')
	h.press('escape')!
	assert h.find_role(.dialog, 'Pod api-7d9f8c') == none
}

fn test_status_toggles_and_settings_json() {
	mut h := testing.new_harness(width: 1200, height: 800)!
	kit.install(mut h.app)!
	install_commands(mut h.app)!
	h.open(pages.Gallery{}, width: 1200, height: 800)!
	assert h.find('nav-start') != none
	h.click('status-pages')! // the status bar toggle hides the page list
	assert h.find('nav-start') == none
	h.click('status-pages')!
	open_page(mut h, 'settings')!
	h.click('set-a11y')!
	h.click('set-json')!
	assert h.find_role(.dialog, 'settings.json') != none
	assert h.texts().any(it.contains('"accessible_mode": true'))
	h.press('escape')!
	h.click('status-search')! // opens the command palette
	assert h.has_text('Đổi giao diện sáng/tối')
}

fn test_dock_overflow_menu_selects() {
	mut h := testing.new_harness(width: 1200, height: 800)!
	kit.install(mut h.app)!
	h.open(pages.Gallery{}, width: 1200, height: 800)!
	open_page(mut h, 'dock')!
	h.click('dock-left-more')!
	h.click('menu-1')! // app.v
	h.frame()
	assert h.has_text('app.v')
}

fn test_hud_reports_frames_cpu_and_memory() {
	mut h := testing.new_pixel_harness()!
	kit.install(mut h.app)!
	h.open(pages.Gallery{ hud: true },
		width:  1200
		height: 800
	)!
	for _ in 0 .. 3 {
		h.advance(500)
	}
	texts := h.texts()
	assert texts.any(it.starts_with('FPS ')) && texts.any(it.starts_with('CPU '))
	ws := texts.filter(it.starts_with('Working set ')) // a real process has one
	assert ws.len == 1 && ws[0].ends_with(' MB')
	assert texts.any(it.starts_with('Private WS ') && it.ends_with(' MB'))
	hud := h.find('hud') or { panic('no hud') }
	assert hud.right() <= 1200 && hud.right() > 1100 && hud.y > 36 // top right, under the titlebar
	out := os.join_path(os.dir(@FILE), '..', '..', '.tmp', 'out')
	h.save_png(os.join_path(out, 'gallery_hud.png'))!
}
