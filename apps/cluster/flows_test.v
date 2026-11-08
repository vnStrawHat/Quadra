// The main flows of apps/cluster in the harness (specs/19): the value view, applying a
// manifest (good and bad), logs following and filtering, metrics, and events.
module main

import quadra.core
import quadra.kit
import quadra.kit.code
import quadra.kit.term
import quadra.testing

fn open_app() !(&testing.Harness, &Cluster, core.Cx) {
	mut h := testing.new_harness(width: 1280, height: 800)!
	kit.install(mut h.app)!
	v := h.open(Cluster{}, width: 1280, height: 800)!
	cx := core.Cx{
		app: h.app
	}
	return h, cx.peek(v.model()), cx
}

fn test_value_view_toggles_and_copies_paths() {
	mut h, c, mut cx := open_app()!
	h.click('resources-row0')!
	h.click('detail-tabs-yaml')!
	vv := cx.peek(c.values.model())
	assert h.has_text('apiVersion:') && h.has_text('labels:')
	h.click('values-row2')! // metadata
	assert vv.selected == '.metadata'
	h.click('values-row2')! // a second click on the selected row closes it
	assert !h.has_text('namespace:')
	h.press('right')! // and the right arrow opens it again
	assert h.has_text('namespace:')
	h.press('down')!
	h.press('ctrl-c')!
	assert h.plat.clipboard == '.metadata.name'
}

fn test_manifest_applies_and_rejects() {
	mut h, c, mut cx := open_app()!
	h.click('nav-Deployment')!
	h.click('resources-row0')!
	h.click('detail-tabs-manifest')!
	r := c.current() or { panic('no deployment') }
	src := r.yaml()
	// Edits in progress survive ticks.
	code.set_text(mut cx, c.manifest, src.replace('replicas: ${r.replicas}', 'replicas: 5'))
	h.advance(1000)
	assert cx.peek(c.manifest.model()).value().contains('replicas: 5')
	h.click('manifest-apply')!
	assert c.apply_note == 'Applied'
	assert (c.current() or { panic('') }).replicas == 5
	assert c.data.events.last().reason == 'ScalingReplicaSet'
	assert h.has_text('5/5') // the table shows it
	// Renaming is refused, and so is YAML that does not parse.
	code.set_text(mut cx, c.manifest, src.replace('name: ${r.name}\n', 'name: other\n'))
	h.click('manifest-apply')!
	assert c.apply_note.starts_with('Error: kind and metadata.name')
	assert h.has_text(c.apply_note)
	code.set_text(mut cx, c.manifest, 'kind: Deployment\n  bad: indent\n')
	h.click('manifest-apply')!
	assert c.apply_note.starts_with('Error: line')
	h.click('manifest-revert')!
	assert cx.peek(c.manifest.model()).value() == (c.current() or { panic('') }).yaml()
}

fn test_logs_follow_and_filter() {
	mut h, c, mut cx := open_app()!
	h.click('resources-row0')!
	h.click('detail-tabs-logs')!
	lv := cx.peek(c.logs.model())
	n0 := lv.store.len()
	assert n0 > 0 && n0 == c.data.written[c.log_pod]
	for _ in 0 .. 9 {
		h.advance(1000)
	}
	h.frame()
	assert lv.store.len() > n0 // ticks append the pod's new lines
	assert lv.follow
	last := lv.store.line(lv.store.len() - 1)
	plain, _ := term.style_spans(last)
	assert h.has_text(plain) // the newest line is in view, without its escapes
	// Scrolling up leaves follow mode; new lines then keep the view still.
	r := h.find('logs') or { panic('no log view') }
	for _ in 0 .. 50 {
		h.advance(1000)
	}
	h.scroll_at(r.x + 200, r.y + 100, 120)
	assert !lv.follow
	y := lv.store.len()
	h.advance(1000)
	h.advance(1000)
	h.frame()
	assert !lv.follow && lv.store.len() >= y
	h.click('logs-follow')!
	assert lv.follow
	// The filter hides other lines.
	h.click('logs-filter')!
	h.type_text('warn')
	assert lv.shown_count() > 0 && lv.shown_count() < lv.store.len()
	assert h.texts().filter(it.contains('GET /api')).all(it.to_lower().contains('warn'))
	h.click('logs-dim')!
	assert lv.shown_count() == lv.store.len()
}

fn test_metrics_and_events() {
	mut h, c, mut cx := open_app()!
	h.click('resources-row0')!
	h.click('detail-tabs-metrics')!
	cpu := h.find('metrics-cpu') or { panic('no cpu chart') }
	assert h.find('metrics-mem') != none
	h.move_to(cpu.x + cpu.width / 2, cpu.y + cpu.height / 2) // the crosshair paints
	h.frame()
	h.click('nav-Deployment')!
	h.click('resources-row0')!
	h.click('detail-tabs-metrics')!
	assert h.find('metrics-bars') != none
	d := c.current() or { panic('') }
	pods := c.data.of('Pod').filter(it.name.all_before('-') == d.name)
	assert pods.len > 0 && h.has_text(pods[0].name)
	// Events: a live table, newest first.
	h.click('act-events')!
	n := c.event_rows.len
	assert h.has_text('Events · ${n}')
	for _ in 0 .. 30 {
		h.advance(1000)
	}
	h.frame()
	assert c.event_rows.len > n
	assert h.has_text('Events · ${c.event_rows.len}')
	assert h.has_text(c.event_rows[0].message)
}
