// Remote directory listing, navigation and following the terminal's folder.
module sftpui

import ssh
import quadra.core
import quadra.kit

// Listed is a finished listing; key and generation drop answers that are no longer wanted.
struct Listed {
	key      u64
	gen      u64
	resolved string
	entries  []ssh.FileEntry
	err      string
}

fn on_path_submit(mut p SftpPanel, v &string, mut cx core.ViewCx[SftpPanel]) {
	path := (*v).trim_space()
	if path != '' {
		p.goto_path(path, mut cx)
	}
}

fn on_path_change(mut p SftpPanel, _ &string, mut cx core.ViewCx[SftpPanel]) {
	if p.active != unsafe { nil } && p.active.path_error {
		p.active.path_error = false
		cx.notify()
	}
}

// load_dir lists path on a worker. A relative path is resolved first, so an empty folder
// still knows its absolute cwd.
fn (mut p SftpPanel) load_dir(path string, mut cx core.ViewCx[SftpPanel]) {
	if p.active == unsafe { nil } {
		return
	}
	mut st := p.active
	st.generation++
	st.cwd, st.error, st.selected, st.loading = path, '', -1, true
	p.sync_table(mut cx)
	p.sync_path(mut cx)
	cx.notify()
	core.spawn_task_with[SftpPanel, Job, Listed](mut cx, Job{
		client: st.client
		key:    st.key
		gen:    st.generation
		path:   path
	}, fn (j Job, _ &core.TaskCx) !Listed {
		mut client := j.client
		path, key, gen := j.path, j.key, j.gen
		mut resolved := ''
		if !path.starts_with('/') {
			resolved = client.realpath(path) or { '' }
		}
		listed := if resolved != '' { resolved } else { path }
		entries := client.read_dir(listed) or {
			return Listed{
				key: key
				gen: gen
				err: err.msg()
			}
		}
		return Listed{
			key:      key
			gen:      gen
			resolved: resolved
			entries:  entries
		}
	}, on_listed)
}

fn on_listed(mut p SftpPanel, r &core.TaskResult[Listed], mut cx core.ViewCx[SftpPanel]) {
	l := r.value
	mut st := p.states[l.key] or { return }
	if st.generation != l.gen {
		return
	}
	st.loading = false
	if l.err != '' {
		st.error = l.err // the previous rows stay under the banner
	} else {
		mut abs_dir := l.resolved
		if abs_dir == '' && l.entries.len > 0 {
			abs_dir = ssh.remote_parent(l.entries[0].path)
		}
		if abs_dir != '' {
			st.cwd = abs_dir
		}
		st.entries = l.entries.clone()
		sort_entries(mut st.entries, st.sort)
	}
	p.sync_table(mut cx)
	p.sync_path(mut cx)
	cx.notify()
}

// sync_table copies the active session into the table.
fn (mut p SftpPanel) sync_table(mut cx core.ViewCx[SftpPanel]) {
	if p.active == unsafe { nil } {
		return
	}
	st := p.active
	mut t := cx.peek(p.table.model())
	t.rows = st.entries.map(row_of(it))
	t.paths = st.entries.map(it.path)
	t.selected, t.sort, t.loading = st.selected, st.sort, st.loading
	cx.touch(p.table.model())
}

fn (mut p SftpPanel) refresh(mut cx core.ViewCx[SftpPanel]) {
	if p.active != unsafe { nil } && p.active.cwd != '' {
		p.load_dir(p.active.cwd, mut cx)
	}
}

fn (mut p SftpPanel) navigate_parent(mut cx core.ViewCx[SftpPanel]) {
	if p.active == unsafe { nil } {
		return
	}
	parent := ssh.remote_parent(p.active.cwd)
	if parent != '' {
		p.load_dir(parent, mut cx)
	}
}

fn (mut p SftpPanel) navigate_into(i int, mut cx core.ViewCx[SftpPanel]) {
	if p.active == unsafe { nil } || i < 0 || i >= p.active.entries.len {
		return
	}
	e := p.active.entries[i]
	if e.is_dir {
		p.load_dir(e.path, mut cx)
	}
}

// goto_path checks the path is a folder, then opens it; otherwise the box turns red.
fn (mut p SftpPanel) goto_path(path string, mut cx core.ViewCx[SftpPanel]) {
	if p.active == unsafe { nil } {
		return
	}
	mut st := p.active
	core.spawn_task_with[SftpPanel, Job, GotoResult](mut cx, Job{
		client: st.client
		key:    st.key
		gen:    st.generation
		path:   path
	}, fn (j Job, _ &core.TaskCx) !GotoResult {
		mut client := j.client
		path, key, gen := j.path, j.key, j.gen
		e := client.stat(path) or { return GotoResult{
			key:  key
			gen:  gen
			path: path
		} }
		return GotoResult{
			key:    key
			gen:    gen
			path:   path
			is_dir: e.is_dir
		}
	}, on_goto)
}

struct GotoResult {
	key    u64
	gen    u64
	path   string
	is_dir bool
}

fn on_goto(mut p SftpPanel, r &core.TaskResult[GotoResult], mut cx core.ViewCx[SftpPanel]) {
	g := r.value
	if p.active == unsafe { nil } || p.active.key != g.key || p.active.generation != g.gen {
		return
	}
	if g.is_dir {
		p.active.path_error = false
		p.load_dir(g.path, mut cx)
	} else {
		p.active.path_error = true
		cx.notify()
	}
}

fn (mut p SftpPanel) sync_to_terminal(mut cx core.ViewCx[SftpPanel]) {
	if p.term_cwd != '' && p.active != unsafe { nil } {
		p.goto_path(p.term_cwd, mut cx)
	}
}

fn (mut p SftpPanel) toggle_follow(mut cx core.ViewCx[SftpPanel]) {
	if p.active == unsafe { nil } {
		return
	}
	p.active.follow = !p.active.follow
	if p.active.follow {
		p.maybe_follow(mut cx)
	}
	cx.notify()
}

// maybe_follow goes to the terminal's folder when Follow is on and it changed.
fn (mut p SftpPanel) maybe_follow(mut cx core.ViewCx[SftpPanel]) {
	if p.active == unsafe { nil } || !p.active.follow || p.term_cwd == '' {
		return
	}
	cwd := p.term_cwd
	mut st := p.active
	if st.last_followed == cwd {
		return
	}
	st.last_followed = cwd
	if st.cwd != cwd {
		p.goto_path(cwd, mut cx)
	}
}

// selected_entry is the entry under the selection, if any.
fn (p &SftpPanel) selected_entry() ?ssh.FileEntry {
	if p.active == unsafe { nil } || p.active.selected < 0
		|| p.active.selected >= p.active.entries.len {
		return none
	}
	return p.active.entries[p.active.selected]
}

fn (mut p SftpPanel) resort(mut cx core.ViewCx[SftpPanel]) {
	if p.active == unsafe { nil } {
		return
	}
	mut st := p.active
	mut path := ''
	if st.selected >= 0 && st.selected < st.entries.len {
		path = st.entries[st.selected].path
	}
	t := cx.peek(p.table.model())
	st.sort = t.sort
	sort_entries(mut st.entries, st.sort)
	st.selected = -1
	if path != '' {
		for i, e in st.entries {
			if e.path == path {
				st.selected = i
			}
		}
	}
	p.sync_table(mut cx)
}

fn (mut p SftpPanel) remote_error_banner() ?string {
	if p.active == unsafe { nil } || p.active.error == '' {
		return none
	}
	return 'Could not open "${p.active.cwd}": ${p.active.error}'
}

// on_remote_table handles the remote table's clicks, drops and menus.
fn on_remote_table(mut p SftpPanel, e &TableEvent, mut cx core.ViewCx[SftpPanel]) {
	if p.active == unsafe { nil } {
		return
	}
	match e.kind {
		.select {
			p.active.selected = e.row
			cx.notify()
		}
		.clear {
			p.active.selected = -1
			cx.notify()
		}
		.open {
			p.navigate_into(e.row, mut cx)
		}
		.context_row {
			p.active.selected = e.row
			target := if p.active.entries[e.row].is_dir {
				MenuTarget.directory
			} else {
				MenuTarget.file
			}
			kit.open_context_menu(mut cx.Cx, e.x, e.y, p.menu_items(target, mut cx))
		}
		.context_empty {
			kit.open_context_menu(mut cx.Cx, e.x, e.y, p.menu_items(.empty_area, mut cx))
		}
		.sorted {
			p.resort(mut cx)
			cx.notify()
		}
		.resized {
			p.schedule_save(mut cx)
		}
		.dropped {
			p.upload_paths([e.path], mut cx)
		}
	}
}
