// The live resource table: columns, the once-a-second tick and selection by key.
module main

import apps.cluster.fake
import quadra.core
import quadra.kit.data

fn (mut c Cluster) setup(mut cx core.ViewCx[Cluster]) {
	// The shell's program ends with the window (a local cmd.exe would outlive it).
	cx.on_close_requested(cx.listener(fn (mut c Cluster, _ &core.CloseRequest, mut cx core.ViewCx[Cluster]) {
		if c.term.id != 0 {
			mut t := cx.peek(c.term.model())
			t.close()
		}
	}))
	// The cell function is made once and reads the rows live: ticks make no closures.
	holder := unsafe { &c }
	c.cell = fn [holder] (row int, col int) string {
		r := holder.rows[row]
		return match col {
			0 { r.name }
			1 { r.namespace }
			2 { r.status }
			3 { r.ready }
			4 { r.restarts.str() }
			5 { r.age_s.str() }
			else { r.node }
		}
	}
	c.table = data.new_table(mut cx.Cx,
		id:      'resources'
		columns: [
			data.Column{
				title: 'Name'
				width: 300
			},
			data.Column{
				title: 'Namespace'
				width: 140
			},
			data.Column{
				title: 'Status'
				width: 130
				kind:  .status
			},
			data.Column{
				title: 'Ready'
				width: 70
			},
			data.Column{
				title:   'Restarts'
				width:   90
				numeric: true
			},
			data.Column{
				title: 'Age'
				width: 70
				kind:  .age
			},
			data.Column{
				title: 'Node'
				width: 110
			},
		]
		cell:    c.cell
	)
	data.on_select(mut cx.Cx, c.table, cx.listener(fn (mut c Cluster, rows &[]int, mut cx core.ViewCx[Cluster]) {
		c.selected = if rows.len == 1 && rows[0] < c.rows.len { c.rows[rows[0]].key() } else { '' }
		c.sync(mut cx)
	}))
	cx.set_interval(1000, fn (mut c Cluster, mut cx core.ViewCx[Cluster]) {
		if c.live {
			c.tick(mut cx)
		}
	})
	c.setup_events(mut cx)
	c.show_kind(c.kind, mut cx)
}

// tick advances the cluster one second and refreshes what shows it.
pub fn (mut c Cluster) tick(mut cx core.ViewCx[Cluster]) {
	c.data.tick()
	c.refresh(mut cx)
	c.refresh_events(mut cx)
	c.sync(mut cx)
	cx.notify()
}

// refresh reloads the current kind into the table, keeping the selection by key.
fn (mut c Cluster) refresh(mut cx core.ViewCx[Cluster]) {
	c.rows = c.data.of(c.kind)
	keys := c.rows.map(it.key())
	if c.selected != '' && c.selected !in keys {
		c.selected = '' // the resource went away
	}
	data.set_keyed_rows(mut cx.Cx, c.table, keys, c.cell)
}

// show_kind loads one kind into the table (from listeners, not render).
fn (mut c Cluster) show_kind(kind string, mut cx core.ViewCx[Cluster]) {
	c.kind = kind
	if kind != 'Pod' && c.tab == 'shell' {
		c.tab = 'overview' // only pods have a shell
	}
	c.selected = ''
	c.refresh(mut cx)
	c.sync(mut cx)
}

// current is the selected resource, if it still exists.
fn (c &Cluster) current() ?fake.Resource {
	if c.selected == '' {
		return none
	}
	for r in c.rows {
		if r.key() == c.selected {
			return r
		}
	}
	return none
}
