// Edit a remote file locally: download to a working copy, open it, watch for saves, upload
// them back (specs/27).
module sftpui

import hash.fnv1a
import os
import os.filelock
import time
import ssh
import quadra.core

// FileSig fingerprints the working copy. The mtime has one-second resolution, so a content
// hash covers the seconds right after a save.
struct FileSig {
	mtime i64
	size  u64
	hash  u64
}

fn (a FileSig) same(b FileSig) bool {
	if a.mtime != b.mtime || a.size != b.size {
		return false
	}
	return a.hash == 0 || b.hash == 0 || a.hash == b.hash
}

fn file_sig(path string) ?FileSig {
	st := os.stat(path) or { return none }
	mut h := u64(0)
	if time.now().unix() - st.mtime <= 2 {
		data := os.read_bytes(path) or { return none }
		h = fnv1a.sum64(data)
	}
	return FileSig{st.mtime, st.size, h}
}

@[heap]
struct EditSession {
mut:
	id        u64
	key       u64
	client    Client
	remote    string
	temp      string
	baseline  ?i64 // the remote mtime known to match the copy
	always    bool // the "always upload" checkbox: this session only
	last_sig  ?FileSig
	seen_sig  ?FileSig // differs from last_sig: a change waiting to settle for a tick
	uploading bool
	pending   bool // a save arrived while an upload or a prompt was busy
	prompt    bool
}

// EditLock is held for the process lifetime: its file lock says this pid's folder is live.
struct EditLock {
mut:
	lock filelock.FileLock
}

fn pid_dir(root string, pid int) string {
	return os.join_path(root, pid.str())
}

fn (mut p SftpPanel) ensure_edit_lock() {
	if p.edit_lock != none || p.edit_dir == '' {
		return
	}
	mut l := filelock.new(pid_dir(p.edit_dir, os.getpid()) + '.lock')
	l.try_acquire()
	p.edit_lock = EditLock{l}
}

// sweep_edit_cache removes the folders of dead processes (start-up, and our own leftovers).
// A folder whose lock another process still holds stays.
fn (mut p SftpPanel) sweep_edit_cache() {
	if p.edit_dir == '' {
		return
	}
	names := os.ls(p.edit_dir) or { return }
	me := os.getpid()
	for n in names {
		path := os.join_path(p.edit_dir, n)
		if !os.is_dir(path) || !n.bytes().all(it.is_digit()) || n.int() == me {
			continue
		}
		mut l := filelock.new(path + '.lock')
		if l.try_acquire() {
			os.rmdir_all(path) or {}
			l.release()
		}
	}
}

fn (mut p SftpPanel) do_edit(mut cx core.ViewCx[SftpPanel]) {
	e := p.selected_entry() or {
		p.notice(.warning, 'Select a file to edit.', mut cx)
		return
	}
	if e.is_dir {
		p.notice(.warning, '"${e.name}" is a folder — open it instead of editing it.', mut cx)
		return
	}
	limit := p.settings.limit
	if limit > 0 && e.size > u64(limit) {
		body := '"${e.name}" is ${format_size(e.size)} — larger than the ${limit / (1024 * 1024)} MB edit limit. Open for editing anyway?'
		p.ask(mut cx, 'Large file', body, 'OK', 'Cancel', false, core.listener_with[SftpPanel, ssh.FileEntry, bool](cx.entity,
			e, fn (mut p SftpPanel, e ssh.FileEntry, yes &bool, mut cx core.ViewCx[SftpPanel]) {
			if *yes {
				p.start_edit(e, mut cx)
			}
		}))
		return
	}
	p.start_edit(e, mut cx)
}

struct EditDownloaded {
	key      u64
	remote   string
	temp     string
	baseline ?i64
	status   TransferStatus
}

// start_edit reuses a session for the same file, or downloads a new working copy.
fn (mut p SftpPanel) start_edit(e ssh.FileEntry, mut cx core.ViewCx[SftpPanel]) {
	if p.active == unsafe { nil } {
		p.notice(.warning, 'No active SFTP connection is available.', mut cx)
		return
	}
	key := p.active.key
	for _, mut s in p.edits {
		if s.key == key && s.remote == e.path {
			p.open_in_editor(s.temp, mut cx) or {}
			return
		}
	}
	if p.edit_dir == '' {
		p.notice(.error, 'No folder is configured for edited files.', mut cx)
		return
	}
	p.ensure_edit_lock()
	p.next_edit++
	id := p.next_edit
	dir := os.join_path(pid_dir(p.edit_dir, os.getpid()), id.str())
	os.mkdir_all(dir) or {
		p.notice(.error, 'Could not create the edit temp directory: ${err}', mut cx)
		return
	}
	temp := os.join_path(dir, sanitize_file_name(e.name))
	mut client := p.active.client
	mut bus := p.bus
	name, remote, baseline := e.name, e.path, e.modified
	core.spawn_task[SftpPanel, EditDownloaded](mut cx, fn [mut client, mut bus, key, name, temp, remote, baseline] (_ &core.TaskCx) !EditDownloaded {
		status := run_transfer(mut client, mut bus, key, .download, name, temp, remote)
		return EditDownloaded{key, remote, temp, baseline, status}
	}, on_edit_downloaded)
}

fn on_edit_downloaded(mut p SftpPanel, r &core.TaskResult[EditDownloaded], mut cx core.ViewCx[SftpPanel]) {
	d := r.value
	dir := os.dir(d.temp)
	if d.status != .completed {
		os.rmdir_all(dir) or {}
		return
	}
	st := p.states[d.key] or {
		os.rmdir_all(dir) or {}
		return
	}
	p.open_in_editor(d.temp, mut cx) or {
		os.rmdir_all(dir) or {}
		return
	}
	id := dir.all_after_last(os.path_separator).u64()
	p.edits[id] = &EditSession{
		id:       id
		key:      d.key
		client:   st.client
		remote:   d.remote
		temp:     d.temp
		baseline: d.baseline
		last_sig: file_sig(d.temp)
	}
	p.ensure_tick(mut cx)
}

fn (mut p SftpPanel) open_in_editor(path string, mut cx core.ViewCx[SftpPanel]) ! {
	s := p.settings
	if s.mode == 'custom' && s.program.trim_space() != '' {
		if l := p.launch {
			l(s.program, s.args, path) or {
				p.notice(.error, 'Could not open the editor: ${err}', mut cx)
				return err
			}
			return
		}
		mut proc := os.new_process(s.program)
		mut argv := s.args.clone()
		argv << path
		proc.set_args(argv)
		proc.run()
		return
	}
	if !cx.open_external(path) {
		p.notice(.error, 'Could not open the editor: failed to open "${path}"', mut cx)
		return error('open failed')
	}
}

// poll_edits sees saves: a changed stamp that held still for a tick is one save.
fn (mut p SftpPanel) poll_edits(mut cx core.ViewCx[SftpPanel]) {
	for _, mut s in p.edits {
		sig := file_sig(s.temp) or { continue }
		last := s.last_sig or { FileSig{} }
		if sig.same(last) {
			s.seen_sig = none
			continue
		}
		seen := s.seen_sig or {
			s.seen_sig = sig
			continue
		}

		if !sig.same(seen) {
			s.seen_sig = sig
			continue
		}
		s.last_sig, s.seen_sig = sig, none
		p.on_saved(mut s, mut cx)
	}
}
