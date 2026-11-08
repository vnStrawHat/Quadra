// Detail tab views: the value view (YAML), the manifest editor with Apply, the log view
// following the pod and the metrics charts. sync runs from listeners and ticks, never in
// render, and makes the views on first use.
module main

import apps.cluster.fake
import quadra.core
import quadra.kit
import quadra.kit.code
import quadra.kit.data
import quadra.kit.viz
import quadra.ui

// sync brings the tab views up to date with the selection and the cluster.
fn (mut c Cluster) sync(mut cx core.ViewCx[Cluster]) {
	if c.tab !in tabs_for(c.kind).map(it.id) {
		c.tab = 'overview'
	}
	r := c.current() or { return }
	key := '${r.kind}/${r.key()}'
	value := data.parse_yaml(r.yaml()) or { data.Value{} }
	if c.values.id == 0 {
		c.values = data.new_value_view(mut cx.Cx, 'values', value)
	} else {
		data.set_value(mut cx.Cx, c.values, value)
	}
	if c.manifest.id == 0 {
		c.manifest = code.new_editor(mut cx.Cx,
			id:   'manifest'
			text: r.yaml()
			path: 'manifest.yaml'
		)
		c.manifest_of, c.apply_note = key, ''
	} else if c.manifest_of != key { // edits in progress survive ticks, not a new selection
		code.set_text(mut cx.Cx, c.manifest, r.yaml())
		c.manifest_of, c.apply_note = key, ''
	}
	if r.kind == 'Pod' {
		c.sync_logs(r, mut cx)
	}
}

// sync_logs appends the pod's new lines (all of them when the pod changes).
fn (mut c Cluster) sync_logs(r fake.Resource, mut cx core.ViewCx[Cluster]) {
	if c.logs.id == 0 {
		c.logs = data.new_log_view(mut cx.Cx, id: 'logs')
	}
	if c.log_pod != r.key() {
		data.clear_logs(mut cx.Cx, c.logs)
		c.log_pod, c.log_seen = r.key(), 0
	}
	lines, total := c.data.new_logs(r.key(), c.log_seen)
	c.log_seen = total
	data.append_logs(mut cx.Cx, c.logs, lines.map(data.LogEntry{it.at_ms, it.text}))
}

fn (mut c Cluster) manifest_tab(t &kit.Theme, mut cx core.ViewCx[Cluster]) core.Element {
	note := if c.apply_note == '' {
		core.Element(ui.text('Edit and apply: image and replicas').color(t.p.text_3))
	} else if c.apply_note.starts_with('Error') {
		core.Element(kit.status_label(.failed, c.apply_note))
	} else {
		core.Element(kit.status_label(.succeeded, c.apply_note))
	}
	bar := ui.div().flex_row().items_center().gap(8).px(8).h(t.panel_header_h).shrink_0().border_b(1).border_color(t.p.line).child(note).child(ui.div().grow()).child(kit.button('manifest-revert').label('Revert').size(.sm).on_click(cx.listener(fn (mut c Cluster, _ &core.ClickEvent, mut cx core.ViewCx[Cluster]) {
		c.manifest_of = ''
		c.sync(mut cx)
	}))).child(kit.button('manifest-apply').label('Apply').primary().size(.sm).on_click(cx.listener(fn (mut c Cluster, _ &core.ClickEvent, mut cx core.ViewCx[Cluster]) {
		c.apply(mut cx)
	})))
	return ui.div().flex_col().size_full().child(bar).child(ui.div().grow().child(cx.child_view(c.manifest)))
}

// apply validates the edited manifest and applies its image and replicas.
fn (mut c Cluster) apply(mut cx core.ViewCx[Cluster]) {
	r := c.current() or { return }
	src := cx.peek(c.manifest.model()).value()
	v := data.parse_yaml(src) or {
		c.apply_note = 'Error: ${err.msg()}'
		return
	}
	name := v.at('.metadata.name') or { data.Value{} }
	if name.text != r.name || (v.at('.kind') or { data.Value{} }).text != r.kind {
		c.apply_note = 'Error: kind and metadata.name cannot change'
		return
	}
	image := v.at('.spec.containers[0].image') or {
		v.at('.spec.template.spec.containers[0].image') or { data.Value{} }
	}
	replicas := (v.at('.spec.replicas') or { data.Value{} }).text.int()
	c.data.apply_manifest(r.kind, r.key(), image.text, replicas) or {
		c.apply_note = 'Error: ${err.msg()}'
		return
	}
	c.apply_note = 'Applied'
	c.refresh(mut cx)
	c.refresh_events(mut cx)
	c.sync(mut cx)
}

// metrics shows CPU and memory over the last two minutes for a pod, and the CPU of each
// of its pods for a deployment.
fn (c &Cluster) metrics(t &kit.Theme, r fake.Resource) core.Element {
	mut col := ui.div().flex_col().gap(16).p(16).w_full()
	if r.kind == 'Deployment' {
		mut bars := []viz.Bar{}
		for p in c.data.of('Pod') {
			if p.app == r.name {
				cpu := c.data.cpu[p.key()] or { []f64{} }
				bars << viz.Bar{p.name, if cpu.len > 0 { cpu.last() } else { 0 }, none}
			}
		}
		return col.child(ui.text('CPU by pod').weight(600)).child(viz.bar_chart('metrics-bars',
			bars).unit('m'))
	}
	cpu := c.data.cpu[r.key()] or { []f64{} }
	mem := c.data.mem[r.key()] or { []f64{} }
	if cpu.len == 0 {
		return col.child(ui.text('No samples: the pod is not running').color(t.p.text_3))
	}
	start := fake.start_ms + i64(c.data.t - cpu.len + 1) * 1000
	return col.child(ui.text('CPU (millicores)').weight(600)).child(viz.line_chart('metrics-cpu', [
		viz.Series{'cpu', cpu, none},
	]).time(start, 1000).unit('m').height(160)).child(ui.text('Memory (MiB)').weight(600)).child(viz.line_chart('metrics-mem', [
		viz.Series{'memory', mem, viz.series_color(t, 1)},
	]).time(start, 1000).unit('Mi').height(160))
}
