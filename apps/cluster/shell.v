// The cluster dashboard (specs/16, specs/19): activity bar, resource kinds, a live resource
// table above a detail pane, status bar. The fake cluster ticks once a second.
module main

import apps.cluster.fake
import quadra.core
import quadra.kit
import quadra.kit.code
import quadra.kit.data
import quadra.kit.work
import quadra.kit.term
import quadra.ui

@[heap]
pub struct Cluster {
pub mut:
	data     &fake.Cluster = fake.new_cluster()
	activity string        = 'workloads'
	kind     string        = 'Pod'
	rows     []fake.Resource
	table    core.View[data.Table]
	selected string // key of the selected resource ('' = none)
	tab      string        = 'overview'
	root     work.PaneNode = work.vsplit(work.leaf('list'), work.leaf('detail'), 0.55)
	live     bool          = true // tick once a second (tests turn it off and call tick)
	// detail views, made on first use
	term        core.View[term.Terminal]
	shell       &term.FakePty = unsafe { nil }
	shell_pod   string
	manifest    core.View[code.Editor]
	manifest_of string // key shown in the editor (edits survive ticks)
	apply_note  string
	values      core.View[data.ValueView]
	logs        core.View[data.LogView]
	log_pod     string
	log_seen    int
	events      core.View[data.Table]
	event_rows  []fake.Event // newest first
}

const kinds = {
	'workloads': ['Pod', 'Deployment']
	'config':    ['ConfigMap']
	'network':   ['Service']
	'events':    []string{}
}

@[markused]
pub fn (mut c Cluster) render(mut cx core.ViewCx[Cluster]) core.Element {
	if c.table.id == 0 {
		c.setup(mut cx)
	}
	t := kit.theme(cx)
	bar := work.activity_bar('act', [
		work.Activity{'workloads', kit.icon_layers, 'Workloads', 0},
		work.Activity{'config', kit.icon_file, 'Config', 0},
		work.Activity{'network', kit.icon_external_link, 'Network', 0},
		work.Activity{'events', kit.icon_bell, 'Events', 0},
	], c.activity).footer([work.Activity{'settings', kit.icon_settings, 'Settings', 0}]).on_select(cx.listener(fn (mut c Cluster, id &string, mut cx core.ViewCx[Cluster]) {
		c.pick_activity(*id, mut cx)
	}))
	mut items := []kit.NavItem{}
	for k in kinds[c.activity] or { []string{} } {
		items << kit.NavItem{k, '${k}s (${c.data.of(k).len})', kit.icon_layers}
	}
	nav := kit.sidebar([kit.NavSection{c.activity, items}], c.kind).on_select(cx.listener(fn (mut c Cluster, id &string, mut cx core.ViewCx[Cluster]) {
		c.show_kind(*id, mut cx)
	}))
	list := ui.div().flex_col().size_full().child(work.panel_header('${c.kind}s · ${c.rows.len}', [
		kit.icon_button('list-refresh', kit.icon_refresh_cw).size(.xs).tooltip('Refresh'),
	])).child(ui.div().grow().child(cx.child_view(c.table)))
	panes := work.panes('main', c.root, {
		'list':   core.Element(list)
		'detail': c.detail(t, mut cx)
	}).on_resize(cx.listener(fn (mut c Cluster, r &work.PaneResize, mut _ core.ViewCx[Cluster]) {
		c.root = c.root.with_ratio(r.path, r.ratio)
	}))
	status := kit.statusbar([kit.status_label(.running, 'Connected'),
		kit.status_item(kit.icon_layers, c.data.name)], [
		kit.status_item('', '${c.data.resources.len} resources'),
	])
	panel := if c.activity == 'events' {
		core.Element(ui.div().flex_col().size_full().child(work.panel_header('Events · ${c.event_rows.len}', [])).child(ui.div().grow().child(cx.child_view(c.events))))
	} else {
		core.Element(panes)
	}
	mut body := ui.div().flex_row().grow().child(bar)
	if items.len > 0 { // events has no kinds: no empty sidebar
		body.child(nav)
	}
	body.child(ui.div().grow().h_full().child(panel))
	return ui.div().flex_col().size_full().child(kit.titlebar('Quadra Cluster').menu(cluster_menu())).child(body).child(status).on_action(cx.listener(fn (mut c Cluster, a &ShowActivity, mut cx core.ViewCx[Cluster]) {
		c.pick_activity(a.id, mut cx)
	}))
}

fn (mut c Cluster) pick_activity(id string, mut cx core.ViewCx[Cluster]) {
	c.activity = id
	ks := kinds[id] or { []string{} }
	if ks.len > 0 {
		c.show_kind(ks[0], mut cx)
	}
}

// ShowActivity switches the activity (the titlebar menu dispatches it).
pub struct ShowActivity {
	id string
}

// ClusterCmd is a titlebar menu entry, closure-free: an activity, or '' for the theme.
struct ClusterCmd {
	activity string
}

fn (m ClusterCmd) run(mut cx core.Cx) {
	if m.activity != '' {
		cx.dispatch(ShowActivity{m.activity})
		return
	}
	dark := kit.theme_of(cx.app).mode == .dark
	kit.set_theme(mut cx.app, if dark { kit.light() } else { kit.dark() })
}

// cluster_menu is the hamburger menu: the activities, then the theme.
fn cluster_menu() []kit.MenuItem {
	mut items := []kit.MenuItem{}
	for id, title in {
		'workloads': 'Workloads'
		'config':    'Config'
		'network':   'Network'
		'events':    'Events'
	} {
		items << kit.MenuItem{
			label:  title
			action: ClusterCmd{id}
		}
	}
	items << kit.separator()
	items << kit.MenuItem{
		label:  'Toggle Theme'
		action: ClusterCmd{''}
	}
	return items
}
