// The Events activity: a live table of cluster events, newest first.
module main

import quadra.core
import quadra.kit.data

fn (mut c Cluster) setup_events(mut cx core.ViewCx[Cluster]) {
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
		cells:   EventCells{unsafe { &c }}
	)
	c.refresh_events(mut cx)
}

fn (mut c Cluster) refresh_events(mut cx core.ViewCx[Cluster]) {
	if c.events.id == 0 {
		return
	}
	c.event_rows = c.data.events.reverse()
	data.set_keyed_rows_from(mut cx.Cx, c.events, c.event_rows.map(it.id.str()), EventCells{unsafe { &c }})
}

// EventCells reads the event rows live (data.Cells): ticks make no closures.
struct EventCells {
	c &Cluster = unsafe { nil }
}

fn (e EventCells) cell(row int, col int) string {
	ev := e.c.event_rows[row]
	return match col {
		0 { ev.typ }
		1 { ev.reason }
		2 { ev.object }
		3 { ev.message }
		else { (e.c.data.t - ev.at).str() }
	}
}
