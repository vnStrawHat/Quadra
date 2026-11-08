// Live data pages: Live table, Log view and Value viewer.
module pages

import quadra.core
import quadra.kit
import quadra.kit.data
import quadra.ui

struct Svc {
mut:
	name    string
	status  string
	latency int
	age     int
}

@[heap]
pub struct LiveTablePage {
mut:
	rows  []Svc
	table core.View[data.Table]
	cell  data.CellFn = unsafe { nil }
	step  int
}

const svc_states = ['Running', 'Running', 'Pending', 'Running', 'Failed']

fn (mut p LiveTablePage) setup(mut cx core.ViewCx[LiveTablePage]) {
	p.rows = []Svc{len: 40, init: Svc{'service-${index:02}', 'Running', 20 + index * 7 % 90, index * 3700}}
	holder := unsafe { &p }
	p.cell = fn [holder] (row int, col int) string {
		s := holder.rows[row]
		return match col {
			0 { s.name }
			1 { s.status }
			2 { s.latency.str() }
			else { s.age.str() }
		}
	}
	p.table = data.new_table(mut cx.Cx,
		id:      'live-table'
		columns: [
			data.Column{
				title: 'Service'
				width: 200
			},
			data.Column{
				title: 'Status'
				width: 130
				kind:  .status
			},
			data.Column{
				title:   'Latency (ms)'
				width:   120
				numeric: true
			},
			data.Column{
				title: 'Age'
				width: 80
				kind:  .age
			},
		]
		cell:    p.cell
	)
	p.push(mut cx)
}

fn (mut p LiveTablePage) push(mut cx core.ViewCx[LiveTablePage]) {
	data.set_keyed_rows(mut cx.Cx, p.table, p.rows.map(it.name), p.cell)
}

// update changes a few rows the same way every time (a stand-in for a live source).
fn (mut p LiveTablePage) update(mut cx core.ViewCx[LiveTablePage]) {
	p.step++
	for k in 0 .. 4 {
		i := (p.step * 7 + k * 11) % p.rows.len
		p.rows[i].latency = 10 + (p.step * 37 + i * 13) % 180
		p.rows[i].status = svc_states[(p.step + i) % svc_states.len]
	}
	for mut r in p.rows {
		r.age += 60
	}
	p.push(mut cx)
}

@[markused]
pub fn (mut p LiveTablePage) render(mut cx core.ViewCx[LiveTablePage]) core.Element {
	t := kit.theme(cx)
	if p.table.id == 0 {
		p.setup(mut cx)
	}
	update := kit.button('live-update').label('Update').on_click(cx.listener(fn (mut p LiveTablePage, _ &core.ClickEvent, mut cx core.ViewCx[LiveTablePage]) {
		p.update(mut cx)
	}))
	return page('Live table',
		'Rows identified by key: an update keeps the selection, the keyboard cursor and the sort, and changed cells flash once. Select a row, sort by latency, then press Update.', [
		demo('Services', col([core.Element(update),
			framed(t, ui.div().h(320).child(cx.child_view(p.table)))]),
			'data.set_keyed_rows(mut cx, table, keys, cell) // cell made once, reads your rows'),
	])
}

pub struct LogViewPage {
mut:
	log  core.View[data.LogView]
	sent int
}

fn sample_lines(from int, n int) []data.LogEntry {
	return []data.LogEntry{len: n, init: data.LogEntry{i64(1793966400000) + i64(from + index) * 250, match (
		from + index) % 9 {
		4 {
			'\x1b[33mWARN \x1b[0m cache miss for key user:${from + index}'
		}
		7 {
			'\x1b[31mERROR\x1b[0m upstream timed out after 3000ms (attempt ${(from + index) % 3 + 1})'
		}
		else {
			'\x1b[32mINFO \x1b[0m GET /api/v1/items/${(from + index) * 7 % 500} 200 ${(from + index) * 13 % 90}ms'
		}
	}}}
}

@[markused]
pub fn (mut p LogViewPage) render(mut cx core.ViewCx[LogViewPage]) core.Element {
	t := kit.theme(cx)
	if p.log.id == 0 {
		p.log = data.new_log_view(mut cx.Cx, id: 'demo-log')
		data.append_logs(mut cx.Cx, p.log, sample_lines(0, 200))
		p.sent = 200
	}
	more := kit.button('log-more').label('Add 1 000 lines').on_click(cx.listener(fn (mut p LogViewPage, _ &core.ClickEvent, mut cx core.ViewCx[LogViewPage]) {
		data.append_logs(mut cx.Cx, p.log, sample_lines(p.sent, 1000))
		p.sent += 1000
	}))
	return page('Log view',
		'An append-only log with ANSI colors, timestamps and follow mode: scroll up to stop following, back to the bottom (or Follow) to resume. The filter hides or dims other lines; .* makes it a regular expression.', [
		demo('Requests', col([core.Element(more),
			framed(t, ui.div().h(340).child(cx.child_view(p.log)))]),
			'data.append_logs(mut cx, log, lines) // no closure per call'),
	])
}

pub struct ValueViewPage {
mut:
	view core.View[data.ValueView]
}

const value_sample = '{"name": "web", "replicas": 3, "ready": true, "selector": null,
 "labels": {"app": "web", "tier": "frontend"},
 "containers": [{"name": "web", "image": "registry.local/web:1.4.2", "ports": [8080, 9090],
   "resources": {"limits": {"cpu": "500m", "memory": "256Mi"}}}]}'

@[markused]
pub fn (mut p ValueViewPage) render(mut cx core.ViewCx[ValueViewPage]) core.Element {
	t := kit.theme(cx)
	if p.view.id == 0 {
		v := data.parse_json(value_sample) or { data.Value{} }
		p.view = data.new_value_view(mut cx.Cx, 'demo-value', v)
	}
	return page('Value viewer',
		'A collapsible tree of a JSON or YAML value. Click or press right and left to open and close; ctrl-c or Copy path copies the selected path.', [
		demo('Deployment (JSON)', framed(t, ui.div().h(340).child(cx.child_view(p.view))),
			'data.new_value_view(mut cx, id, data.parse_json(s)!)  // or parse_yaml'),
	])
}
