// Column state and the dual-pane fields, kept in the app's docks.json under sftp_table_state.
module sftpui

import os
import x.json2
import quadra.core

fn (mut p SftpPanel) load_state(mut cx core.ViewCx[SftpPanel]) {
	if p.state_path == '' {
		return
	}
	text := os.read_file(p.state_path) or { return }
	raw := json2.decode[json2.Any](text) or { return }
	st := raw.as_map()['sftp_table_state'] or { return }
	m := st.as_map()
	widths := if v := m['column_widths'] {
		v.as_map()
	} else {
		map[string]json2.Any{}
	}
	vis := if v := m['column_visibility'] {
		v.as_map()
	} else {
		map[string]json2.Any{}
	}
	mut t := cx.peek(p.table.model())
	for i, c in t.cols {
		if c.sort == .name {
			continue
		}
		if w := widths[c.key] {
			v := w.f32()
			if v >= column_min_width && v <= column_max_width {
				t.cols[i].width = v
			}
		}
		if v := vis[c.key] {
			t.cols[i].visible = v.bool()
		}
	}
	if d := m['local_dir'] {
		if d.str() != '' {
			p.local.start = d.str()
		}
	}
	if e := m['expanded'] {
		if e.bool() {
			p.set_expanded(true, mut cx)
		}
	}
}

fn (mut p SftpPanel) schedule_save(mut cx core.ViewCx[SftpPanel]) {
	if p.state_path == '' {
		return
	}
	cx.debounce('sftp-state', 1000, fn (mut p SftpPanel, mut cx core.ViewCx[SftpPanel]) {
		p.save_state(mut cx)
	})
}

// save_state merges this feature's field into docks.json, keeping everything else.
fn (mut p SftpPanel) save_state(mut cx core.ViewCx[SftpPanel]) {
	mut root := map[string]json2.Any{}
	if text := os.read_file(p.state_path) {
		if raw := json2.decode[json2.Any](text) {
			root = raw.as_map().clone()
		}
	}
	mut widths := map[string]json2.Any{}
	mut vis := map[string]json2.Any{}
	for c in cx.peek(p.table.model()).cols {
		if c.sort != .name {
			widths[c.key] = json2.Any(f64(c.width))
		}
		vis[c.key] = json2.Any(c.visible)
	}
	mut st := map[string]json2.Any{}
	st['column_widths'] = json2.Any(widths)
	st['column_visibility'] = json2.Any(vis)
	st['expanded'] = json2.Any(p.expanded)
	st['local_dir'] = json2.Any(p.local.cwd)
	root['sftp_table_state'] = json2.Any(st)
	os.write_file(p.state_path, json2.encode(root)) or {}
}
