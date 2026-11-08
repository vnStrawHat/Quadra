// Workbench pages (specs/16): activity bar, tab strip, panel header, pane tree.
module pages

import quadra.core
import quadra.kit
import quadra.kit.work
import quadra.ui

fn activities() []work.Activity {
	return [work.Activity{'workloads', kit.icon_layers, 'Workloads', 0},
		work.Activity{'config', kit.icon_file, 'Config', 0},
		work.Activity{'search', kit.icon_search, 'Search', 0},
		work.Activity{'events', kit.icon_bell, 'Events', 12}]
}

pub struct ActivityPage {
mut:
	active string = 'workloads'
}

@[markused]
pub fn (mut p ActivityPage) render(mut cx core.ViewCx[ActivityPage]) core.Element {
	t := kit.theme(cx)
	bar := work.activity_bar('wb-act', activities(), p.active).footer([
		work.Activity{'settings', kit.icon_settings, 'Settings', 0},
	]).on_select(cx.listener(fn (mut p ActivityPage, id &string, mut _ core.ViewCx[ActivityPage]) {
		p.active = *id
	}))
	return page('Activity bar',
		'The leftmost column: one icon per tool view, the active one in the accent, badges for counts.', [
		demo('Cluster tools', framed(t, ui.div().flex_row().h(280).child(bar).child(ui.div().grow().p(16).child(hint(t,
			'Active: ${p.active}')))),
			"work.activity_bar('act', items, v.active).footer([settings]).on_select(...)"),
	])
}

pub struct TabStripPage {
mut:
	tabs   []work.Tab = [work.Tab{'deploy', 'api.yaml', kit.icon_file, false},
		work.Tab{'logs', 'Logs: api-7d9f8c', kit.icon_list, false},
		work.Tab{'shell', 'Shell', kit.icon_monitor, false},
		work.Tab{'main', 'main.v', kit.icon_file, true}]
	active string = 'logs'
}

@[markused]
pub fn (mut p TabStripPage) render(mut cx core.ViewCx[TabStripPage]) core.Element {
	t := kit.theme(cx)
	strip := work.tab_strip('wb-tabs', p.tabs, p.active).on_select(cx.listener(fn (mut p TabStripPage, id &string, mut _ core.ViewCx[TabStripPage]) {
		p.active = *id
	})).on_close(cx.listener(fn (mut p TabStripPage, id &string, mut _ core.ViewCx[TabStripPage]) {
		p.tabs = p.tabs.filter(it.id != *id)
	})).on_move(cx.listener(fn (mut p TabStripPage, m &work.TabMove, mut _ core.ViewCx[TabStripPage]) {
		i := p.tabs.map(it.id).index(m.id)
		tab := p.tabs[i]
		p.tabs.delete(i)
		p.tabs.insert(m.to, tab)
	}))
	return page('Tab strip',
		'Open editors and views of one pane. Drag to reorder, middle-click or × to close; a dot marks unsaved changes.', [
		demo('Editor tabs', framed(t, ui.div().flex_col().h(160).child(strip).child(ui.div().grow().p(16).bg(t.p.canvas).child(mono(t,
			p.active)))),
			"work.tab_strip('tabs', tabs, v.active).on_select(...).on_close(...).on_move(...)"),
	])
}

fn panel_header_page(t &kit.Theme) core.Element {
	actions := [
		core.Element(kit.icon_button('ph-filter', kit.icon_filter).size(.xs).tooltip('Filter')),
		kit.icon_button('ph-refresh', kit.icon_refresh_cw).size(.xs).tooltip('Refresh'),
		kit.icon_button('ph-more', kit.icon_ellipsis).size(.xs).tooltip('More'),
	]
	return page('Panel header', 'The title strip of a panel: a section label and quiet actions.', [
		demo('Explorer', framed(t, ui.div().flex_col().w(320).child(work.panel_header('Pods',
			actions)).child(ui.div().h(80).p(12).child(hint(t, '124 pods')))),
			"work.panel_header('Pods', [kit.icon_button(...)])"),
	])
}

pub struct PanesPage {
mut:
	root work.PaneNode = work.hsplit(work.leaf('explorer'), work.vsplit(work.leaf('editor'),
		work.leaf('terminal'), 0.65), 0.28)
}

@[markused]
pub fn (mut p PanesPage) render(mut cx core.ViewCx[PanesPage]) core.Element {
	t := kit.theme(cx)
	folded := p.root.is_collapsed('terminal')
	fold := kit.icon_button('wb-fold', if folded {
		kit.icon_chevron_up
	} else {
		kit.icon_chevron_down
	}).size(.xs).tooltip(if folded {
		'Expand'
	} else {
		'Collapse'
	}).on_click(cx.listener(fn (mut p PanesPage, _ &core.ClickEvent, mut _ core.ViewCx[PanesPage]) {
		p.root = p.root.toggle('terminal')
	}))
	mut leaves := map[string]core.Element{}
	for g in p.root.groups() {
		leaves[g] = ui.div().flex_col().size_full().bg(if g == 'editor' {
			t.p.canvas
		} else {
			t.p.panel
		}).child(work.panel_header(g, if g == 'terminal' {
			[core.Element(fold)]
		} else {
			[]core.Element{}
		})).child(ui.div().p(12).child(hint(t, 'pane "${g}"')))
	}
	tree := work.panes('wb-panes', p.root, leaves).on_resize(cx.listener(fn (mut p PanesPage, r &work.PaneResize, mut _ core.ViewCx[PanesPage]) {
		p.root = p.root.with_ratio(r.path, r.ratio)
	}))
	tree.header(t.panel_header_h)
	split := kit.button('wb-split').label('Split editor').icon(kit.icon_panel_left).size(.sm).on_click(cx.listener(fn (mut p PanesPage, _ &core.ClickEvent, mut _ core.ViewCx[PanesPage]) {
		p.root = p.root.split('editor', .row, 'editor ${p.root.groups().len}', false)
	}))
	return page('Pane tree',
		'Nested splits whose leaves are tab groups. The tree is plain data: keep it in your model and save it with your settings.', [
		demo('Workbench', col([row([split, hint(t, '${p.root.groups().len} panes')]),
			framed(t, tree).h(320)]),
			"work.panes('main', v.root, leaves).on_resize(...).header(28)  // v.root.toggle('terminal') folds a leaf"),
	])
}

pub struct DockPage {
mut:
	dock work.Dock = work.Dock{
		root:   work.hsplit(work.leaf('left'), work.leaf('right'), 0.5)
		tabs:   {
			'left':  [work.Tab{'main', 'main.v', kit.icon_file, false},
				work.Tab{'app', 'app.v', kit.icon_file, false}]
			'right': [work.Tab{'logs', 'Logs', kit.icon_list, false},
				work.Tab{'shell', 'Shell', kit.icon_monitor, false}]
		}
		active: {
			'left':  'main'
			'right': 'logs'
		}
	}
}

@[markused]
pub fn (mut p DockPage) render(mut cx core.ViewCx[DockPage]) core.Element {
	t := kit.theme(cx)
	on_dock := cx.listener(fn (mut p DockPage, e &work.DockEvent, mut _ core.ViewCx[DockPage]) {
		p.dock.apply(e)
	})
	mut leaves := map[string]core.Element{}
	for g in p.dock.root.groups() {
		strip := work.tab_strip('dock-${g}', p.dock.tabs[g], p.dock.active[g]).dock(g, on_dock).on_select(core.listener_with[DockPage, string, string](cx.entity,
			g, fn (mut p DockPage, g string, id &string, mut _ core.ViewCx[DockPage]) {
			p.dock.active[g] = *id
		})).on_move(core.listener_with[DockPage, string, work.TabMove](cx.entity, g, fn (mut p DockPage, g string, m &work.TabMove, mut _ core.ViewCx[DockPage]) {
			p.dock.move(g, m)
		}))
		leaves[g] = ui.div().flex_col().size_full().bg(t.p.canvas).child(strip).child(ui.div().p(12).child(mono(t,
			p.dock.active[g])))
	}
	tree := work.panes('dock-panes', p.dock.root, leaves).on_dock(on_dock).on_resize(cx.listener(fn (mut p DockPage, r &work.PaneResize, mut _ core.ViewCx[DockPage]) {
		p.dock.root = p.dock.root.with_ratio(r.path, r.ratio)
	}))
	return page('Dock',
		'Tear a tab off its strip (drag it down) and drop it on another strip, the middle of a pane, or an edge to split there.', [
		demo('Docking workbench', framed(t, tree).h(320),
			"work.panes('main', d.root, leaves).on_dock(l)  // strips: .dock(group, l); l calls d.apply(e)"),
	])
}
