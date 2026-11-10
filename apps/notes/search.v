// Search as you type: each query runs in a task over the index; a newer query cancels the
// older one.
module main

import apps.notes.store
import quadra.core

pub fn search(mut cx core.Cx, m core.Model[Notes], q string) {
	cx.update_with(m, q, fn (mut n Notes, q string, mut cx core.ViewCx[Notes]) {
		n.query = q
		if n.search != 0 {
			cx.cancel_task(n.search)
		}
		n.search = core.spawn_task_with[Notes, Query, []string](mut cx, Query{n.index, q}, fn (j Query, tc &core.TaskCx) ![]string {
			return j.ix.query(j.q, tc)
		}, fn (mut n Notes, r &core.TaskResult[[]string], mut _ core.ViewCx[Notes]) {
			n.search = 0
			if r.ok {
				n.results = r.value
			}
		})
	})
}

struct Query {
	ix store.Index
	q  string
}

// title_of is the title shown for a note: its first line, or a placeholder.
pub fn title_of(n store.Note, untitled string) string {
	t := n.title()
	return if t == '' { untitled } else { t }
}

// preview is the start of the second non-empty line, for the list.
pub fn preview(n store.Note) string {
	mut seen := false
	for line in n.body.split_into_lines() {
		t := line.trim_space()
		if t == '' {
			continue
		}
		if seen {
			return store.clip(t, 90)
		}
		seen = true
	}
	return ''
}
