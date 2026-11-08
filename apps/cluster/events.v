// The Events activity: a live table of cluster events, newest first.
module main

import quadra.core
import quadra.kit.data

fn (mut c Cluster) setup_events(mut cx core.ViewCx[Cluster]) {
	holder := unsafe { &c }
	c.event_cell = fn [holder] (row int, col int) string {
		e := holder.event_rows[row]
		return match col {
			0 { e.typ }
			1 { e.reason }
			2 { e.object }
			3 { e.message }
			else { (holder.data.t - e.at).str() }
		}
	}
	c.events = data.new_table(mut cx.Cx,
		id:      'events'
		columns: [
			data.Column{
				title: 'Type'
				width: 90
			},
			data.Column{
				title: 'Reason'
				width: 150
			},
			data.Column{
				title: 'Object'
				width: 300
			},
			data.Column{
				title: 'Message'
				width: 520
			},
			data.Column{
				title: 'Age'
				width: 70
				kind:  .age
			},
		]
		cell:    c.event_cell
	)
	c.refresh_events(mut cx)
}

fn (mut c Cluster) refresh_events(mut cx core.ViewCx[Cluster]) {
	if c.events.id == 0 {
		return
	}
	c.event_rows = c.data.events.reverse()
	data.set_keyed_rows(mut cx.Cx, c.events, c.event_rows.map(it.id.str()), c.event_cell)
}
