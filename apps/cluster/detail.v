// The detail pane for the selected resource: Overview, YAML, Manifest, Logs, Metrics and
// Shell (which tabs depends on the kind). tabs.v keeps the tab views in sync.
module main

import apps.cluster.fake
import quadra.core
import quadra.kit
import quadra.kit.viz
import quadra.kit.work
import quadra.ui

// tabs_for lists the detail tabs of a kind.
fn tabs_for(kind string) []work.Tab {
	mut tabs := [work.Tab{'overview', 'Overview', kit.icon_info, false},
		work.Tab{'yaml', 'YAML', kit.icon_list, false},
		work.Tab{'manifest', 'Manifest', kit.icon_pencil, false}]
	if kind == 'Pod' {
		tabs << work.Tab{'logs', 'Logs', kit.icon_file, false}
	}
	if kind in ['Pod', 'Deployment'] {
		tabs << work.Tab{'metrics', 'Metrics', kit.icon_gauge, false}
	}
	if kind == 'Pod' {
		tabs << work.Tab{'shell', 'Shell', kit.icon_terminal, false}
	}
	return tabs
}

fn (mut c Cluster) detail(t &kit.Theme, mut cx core.ViewCx[Cluster]) core.Element {
	folded := c.root.is_collapsed('detail')
	fold := kit.icon_button('detail-fold', if folded {
		kit.icon_chevron_up
	} else {
		kit.icon_chevron_down
	}).size(.xs).tooltip(if folded {
		'Expand'
	} else {
		'Collapse'
	}).on_click(cx.listener(fn (mut c Cluster, _ &core.ClickEvent, mut _ core.ViewCx[Cluster]) {
		c.root = c.root.toggle('detail')
	}))
	strip := work.tab_strip('detail-tabs', tabs_for(c.kind), c.tab).actions([fold]).on_select(cx.listener(fn (mut c Cluster, id &string, mut cx core.ViewCx[Cluster]) {
		c.tab = *id
		c.sync(mut cx)
	}))
	col := ui.div().flex_col().size_full().child(strip)
	// The strip stays when nothing is selected, so a folded pane can always be opened again.
	r := c.current() or {
		return col.child(ui.div().grow().bg(t.p.canvas).center().child(ui.text('Select a resource').color(t.p.text_3)))
	}
	body := match c.tab {
		'shell' {
			c.open_shell(r, mut cx)
			core.Element(ui.div().size_full().p(6).bg(t.code.terminal.background).child(cx.child_view(c.term)))
		}
		'yaml' {
			cx.child_view(c.values)
		}
		'manifest' {
			c.manifest_tab(t, mut cx)
		}
		'logs' {
			cx.child_view(c.logs)
		}
		'metrics' {
			core.Element(ui.div().size_full().overflow_y_scroll().child(c.metrics(t, r)))
		}
		else {
			core.Element(ui.div().size_full().overflow_y_scroll().child(c.overview(t, r)))
		}
	}

	return col.child(ui.div().grow().bg(t.p.canvas).child(body))
}

fn field(t &kit.Theme, label string, value core.Element) core.Element {
	return ui.div().flex_row().items_center().py(6).border_t(1).border_color(t.p.line).child(ui.div().w(140).shrink_0().child(ui.text(label).color(t.p.text_3))).child(value)
}

// overview lists the resource's fields, with CPU and memory trends for pods.
fn (c &Cluster) overview(t &kit.Theme, r fake.Resource) core.Element {
	fields := [['Name', r.name], ['Namespace', r.namespace], ['Kind', r.kind],
		['Node', r.node], ['Image', r.image], ['Restarts', if r.kind == 'Pod' {
			r.restarts.str()
		} else {
			''
		}], ['Age', fake.age(r.age_s)]]
	mut col := ui.div().flex_col().p(16).max_w(640)
	col.child(ui.div().flex_row().items_center().py(6).child(ui.div().w(140).child(ui.text('Status').color(t.p.text_3))).child(kit.status_label(kit.status_of(r.status),
		r.status)))
	for f in fields {
		if f[1] != '' {
			col.child(field(t, f[0], ui.text(f[1]).family(t.mono_family).selectable()))
		}
	}
	cpu := c.data.cpu[r.key()] or { []f64{} }
	mem := c.data.mem[r.key()] or { []f64{} }
	if cpu.len > 0 {
		col.child(field(t, 'CPU', ui.div().flex_row().items_center().gap(12).child(viz.sparkline(cpu).size(160,
			22)).child(ui.text('${int(cpu.last())}m').family(t.mono_family))))
		col.child(field(t, 'Memory', ui.div().flex_row().items_center().gap(12).child(viz.sparkline(mem).size(160,
			22).color(viz.series_color(t, 1))).child(ui.text('${int(mem.last())}Mi').family(t.mono_family))))
	}
	return col
}
