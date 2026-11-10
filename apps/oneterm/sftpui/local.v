// The Local pane of the expanded browser: listing and navigation of this machine's folders.
module sftpui

import os
import ssh
import quadra.core
import quadra.kit
import quadra.kit.input

struct LocalState {
mut:
	cwd        string
	start      string
	loaded     bool
	entries    []ssh.FileEntry
	sort       SortState
	selected   int = -1
	loading    bool
	error      string
	path_error bool
	generation u64
	table      core.View[FileTable]
	path       core.View[input.TextInput]
	form_modal &ModalRef = unsafe { nil }
}

struct LocalListed {
	gen     u64
	dir     string
	entries []ssh.FileEntry
	err     string
}

// read_local_dir lists a folder; an entry whose stat fails is still listed (worker thread).
fn read_local_dir(path string) !LocalListed {
	dir := os.real_path(path)
	names := os.ls(dir)!
	mut out := []ssh.FileEntry{cap: names.len}
	for name in names {
		full := os.join_path(dir, name)
		st := os.stat(full) or {
			out << ssh.FileEntry{
				name: name
				path: full
			}
			continue
		}
		out << ssh.FileEntry{
			name:        name
			path:        full
			is_dir:      os.is_dir(full)
			size:        st.size
			modified:    st.mtime
			permissions: st.mode
		}
	}
	return LocalListed{
		dir:     dir
		entries: out
	}
}

fn (mut p SftpPanel) init_local(mut cx core.ViewCx[SftpPanel]) {
	p.local.table = new_table(mut cx.Cx,
		id:        'sftp-local-table'
		cols:      local_columns()
		drag_kind: 'sftp-local'
		drop_kind: 'sftp-remote'
		on_event:  cx.listener(on_local_table)
	)
	p.local.path = input.new_input(mut cx.Cx,
		id:          'sftp-local-path'
		bare:        true
		placeholder: 'Local path'
	)
	input.on_submit(mut cx.Cx, p.local.path, cx.listener(on_local_path_submit))
	input.on_change(mut cx.Cx, p.local.path, cx.listener(fn (mut p SftpPanel, _ &string, mut cx core.ViewCx[SftpPanel]) {
		if p.local.path_error {
			p.local.path_error = false
			cx.notify()
		}
	}))
}

fn (mut p SftpPanel) local_ensure_loaded(mut cx core.ViewCx[SftpPanel]) {
	if !p.local.loaded {
		p.local_load(p.local.start, mut cx)
	}
}

fn (mut p SftpPanel) local_load(path string, mut cx core.ViewCx[SftpPanel]) {
	p.local.loaded = true
	p.local.generation++
	gen := p.local.generation
	p.local.cwd, p.local.error, p.local.selected, p.local.loading = path, '', -1, true
	p.local_sync_table(mut cx)
	p.local_sync_path(mut cx)
	cx.notify()
	core.spawn_task_with[SftpPanel, LocalList, LocalListed](mut cx, LocalList{path, gen}, fn (j LocalList, _ &core.TaskCx) !LocalListed {
		gen := j.gen
		l := read_local_dir(j.path) or { return LocalListed{
			gen: gen
			err: err.msg()
		} }
		return LocalListed{
			gen:     gen
			dir:     l.dir
			entries: l.entries
		}
	}, on_local_listed)
}

// LocalList is a local listing's data (no client involved).
struct LocalList {
	path string
	gen  u64
}

fn on_local_listed(mut p SftpPanel, r &core.TaskResult[LocalListed], mut cx core.ViewCx[SftpPanel]) {
	l := r.value
	if l.gen != p.local.generation {
		return
	}
	p.local.loading = false
	if l.err != '' {
		p.local.error = l.err
	} else {
		p.local.cwd = l.dir
		p.local.entries = l.entries.clone()
		sort_entries(mut p.local.entries, p.local.sort)
		p.schedule_save(mut cx)
	}
	p.local_sync_table(mut cx)
	p.local_sync_path(mut cx)
	cx.notify()
}

fn (mut p SftpPanel) local_sync_table(mut cx core.ViewCx[SftpPanel]) {
	mut t := cx.peek(p.local.table.model())
	t.rows = p.local.entries.map(row_of(it))
	t.paths = p.local.entries.map(it.path)
	t.selected, t.sort, t.loading = p.local.selected, p.local.sort, p.local.loading
	cx.touch(p.local.table.model())
}

fn (mut p SftpPanel) local_sync_path(mut cx core.ViewCx[SftpPanel]) {
	mut w := cx.peek(p.local.path.model())
	if !w.focus.is_focused(cx.Cx) && w.value() != p.local.cwd {
		input.set_value(mut cx.Cx, p.local.path, p.local.cwd)
	}
}

fn (mut p SftpPanel) local_refresh(mut cx core.ViewCx[SftpPanel]) {
	if p.local.cwd != '' {
		p.local_load(p.local.cwd, mut cx)
	} else {
		p.local_ensure_loaded(mut cx)
	}
}

fn (mut p SftpPanel) local_parent(mut cx core.ViewCx[SftpPanel]) {
	parent := os.dir(p.local.cwd)
	if parent != '' && parent != p.local.cwd {
		p.local_load(parent, mut cx)
	}
}

fn on_local_path_submit(mut p SftpPanel, v &string, mut cx core.ViewCx[SftpPanel]) {
	path := (*v).trim_space()
	if path == '' {
		return
	}
	if os.is_dir(path) {
		p.local.path_error = false
		p.local_load(path, mut cx)
	} else {
		p.local.path_error = true
	}
	cx.notify()
}

fn (p &SftpPanel) local_selected() ?ssh.FileEntry {
	if p.local.selected < 0 || p.local.selected >= p.local.entries.len {
		return none
	}
	return p.local.entries[p.local.selected]
}

fn (mut p SftpPanel) local_upload_selected(mut cx core.ViewCx[SftpPanel]) {
	e := p.local_selected() or {
		p.notice(.warning, 'Select a local file or folder to upload.', mut cx)
		return
	}
	p.upload_paths([e.path], mut cx)
}

// on_local_table: double click enters a folder or uploads a file; a remote row dropped here
// downloads into this folder.
fn on_local_table(mut p SftpPanel, e &TableEvent, mut cx core.ViewCx[SftpPanel]) {
	match e.kind {
		.select {
			p.local.selected = e.row
			cx.notify()
		}
		.clear {
			p.local.selected = -1
			cx.notify()
		}
		.open {
			p.local.selected = e.row
			entry := p.local.entries[e.row]
			if entry.is_dir {
				p.local_load(entry.path, mut cx)
			} else {
				p.local_upload_selected(mut cx)
			}
		}
		.context_row {
			p.local.selected = e.row
			kit.open_context_menu(mut cx.Cx, e.x, e.y, p.local_row_menu(p.local.entries[e.row].is_dir, mut
				cx))
		}
		.context_empty {
			kit.open_context_menu(mut cx.Cx, e.x, e.y, p.local_empty_menu(mut cx))
		}
		.sorted {
			p.local_resort(mut cx)
			cx.notify()
		}
		.resized {
			p.schedule_save(mut cx)
		}
		.dropped {
			if p.active != unsafe { nil } {
				for en in p.active.entries {
					if en.path == e.path {
						p.download_to_local(en, mut cx)
						return
					}
				}
			}
		}
	}
}

fn (mut p SftpPanel) local_resort(mut cx core.ViewCx[SftpPanel]) {
	mut path := ''
	if p.local.selected >= 0 && p.local.selected < p.local.entries.len {
		path = p.local.entries[p.local.selected].path
	}
	p.local.sort = cx.peek(p.local.table.model()).sort
	sort_entries(mut p.local.entries, p.local.sort)
	p.local.selected = -1
	if path != '' {
		for i, e in p.local.entries {
			if e.path == path {
				p.local.selected = i
			}
		}
	}
	p.local_sync_table(mut cx)
}
