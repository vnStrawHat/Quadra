module main

import os
import quadra.core
import quadra.kit
import quadra.testing

fn test_browse_and_inspect() {
	mut h := testing.new_harness(width: 1280, height: 800)!
	kit.install(mut h.app)!
	h.open(Cluster{}, width: 1280, height: 800)!
	assert h.has_text('Pods · 124')
	assert h.has_text('Select a resource')
	h.click('resources-row0')!
	assert !h.has_text('Select a resource')
	assert h.has_text('Image')
	h.click('detail-tabs-yaml')! // the value tree of the manifest
	assert h.has_text('kind:') && h.has_text('"Pod"')
	first := h.texts().filter(it.starts_with('"') && it.contains('-7d9f8c6b5-'))
	h.click('resources-row1')! // the tree follows the selection
	assert h.texts().filter(it.starts_with('"') && it.contains('-7d9f8c6b5-')) != first
	h.click('detail-tabs-logs')!
	assert !h.has_text('kind:')
	h.click('nav-Deployment')!
	assert h.has_text('Deployments · 10')
	assert h.has_text('Select a resource')
	h.click('act-network')!
	assert h.has_text('Services · 10')
	h.click('act-config')!
	assert h.has_text('ConfigMaps · 10')
	h.click('titlebar-menu')! // the hamburger menu switches activities too
	h.click('menu-2')! // Network
	assert h.has_text('Services · 10')
	h.click('titlebar-menu')!
	h.click('menu-1')! // Config
	assert h.has_text('ConfigMaps · 10')
	// The detail pane folds to its tab strip and opens again.
	y0 := (h.find('detail-tabs') or { panic('no strip before fold') }).y
	h.click('detail-fold')!
	y1 := (h.find('detail-tabs') or { panic('no strip after fold') }).y
	assert y1 > y0 + 200, 'strip ${y0} -> ${y1}'
	h.click('detail-fold')!
	assert (h.find('detail-tabs') or { panic('') }).y == y0
}

fn test_cluster_pixels() {
	out := os.join_path(os.dir(@FILE), '..', '..', '.tmp', 'out')
	os.mkdir_all(out)!
	mut h := testing.new_pixel_harness()!
	kit.install(mut h.app)!
	h.open(Cluster{}, width: 1280, height: 800)!
	h.click('resources-row2')!
	h.save_png(os.join_path(out, 'cluster.png'))!
	for tab in ['metrics', 'logs', 'yaml'] {
		h.click('detail-tabs-' + tab)!
		h.save_png(os.join_path(out, 'cluster_' + tab + '.png'))!
	}
	h.click('detail-tabs-overview')!
	h.click('detail-fold')!
	h.save_png(os.join_path(out, 'cluster_folded.png'))!
}

fn test_shell_tab_runs_a_container_shell() {
	mut h := testing.new_harness(width: 1280, height: 800)!
	kit.install(mut h.app)!
	v := h.open(Cluster{}, width: 1280, height: 800)!
	h.click('resources-row0')!
	h.click('detail-tabs-shell')!
	mut cx := core.Cx{
		app: h.app
	}
	c := cx.peek(v.model())
	pod := c.rows[0].name
	h.click('shell')!
	h.type_text('hostname')
	h.press('enter')!
	t := cx.peek(c.term.model())
	assert t.screen.text().contains('root@${pod}:/# hostname\n${pod}')
	// Another pod gets a fresh session.
	h.click('resources-row1')!
	assert c.shell_pod == c.rows[1].name
	assert !t.screen.text().contains('hostname')
	h.click('act-network')!
	assert !h.texts().any(it == 'Shell') // services have no shell
	assert c.tab == 'overview'
}

fn test_activity_tooltip_is_one_and_beside_its_item() {
	mut h := testing.new_harness(width: 1280, height: 800)!
	kit.install(mut h.app)!
	h.open(Cluster{}, width: 1280, height: 800)!
	item := h.find('act-network') or { panic('no item') }
	h.move_to(item.x + item.width / 2, item.y + item.height / 2)
	h.advance(600)
	tip := h.text_bounds('Network') or { panic('no tooltip') }
	assert !h.has_text('Config') && !h.has_text('Events') // only the hovered item's
	assert tip.x > item.right() // right of the bar, not over the item above
	assert tip.y > item.y && tip.bottom() < item.bottom()
	nav := h.find_role(.tree, '') or { panic('no sidebar') }
	table := h.find('resources') or { panic('no table') }
	assert nav.bounds.right() <= table.x // the sidebar ends where the table starts
}
