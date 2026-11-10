// Upload and download: pickers, the overwrite question, batches on worker tasks.
module sftpui

import os
import ssh
import quadra.core
import quadra.platform
import quadra.prim

// BatchDone ends a batch; the listing refreshes when its session is still shown.
struct BatchDone {
	key u64
}

struct Existing {
	key     u64
	paths   []string
	entries []ssh.FileEntry
}

// pick_and_upload shows the native file or folder picker (modal), then uploads.
fn (mut p SftpPanel) pick_and_upload(folders bool, mut cx core.ViewCx[SftpPanel]) {
	if p.active == unsafe { nil } {
		p.notice(.warning, 'No active SFTP connection is available.', mut cx)
		return
	}
	title := if folders { 'Select a folder to upload' } else { 'Select files to upload' }
	paths := cx.pick_files(platform.FileDialog{
		title:    title
		folders:  folders
		multiple: !folders
	})
	if paths.len > 0 {
		p.upload_paths(paths, mut cx)
	}
}

// upload_paths lists the remote folder first: what is there decides what would be overwritten.
// A folder that cannot be listed does not stop the upload.
fn (mut p SftpPanel) upload_paths(paths []string, mut cx core.ViewCx[SftpPanel]) {
	if p.active == unsafe { nil } || paths.len == 0 {
		return
	}
	core.spawn_task_with[SftpPanel, Job, Existing](mut cx, Job{
		client: p.active.client
		key:    p.active.key
		cwd:    p.active.cwd
		paths:  paths
	}, fn (j Job, _ &core.TaskCx) !Existing {
		mut client := j.client
		entries := client.read_dir(j.cwd) or { []ssh.FileEntry{} }
		return Existing{j.key, j.paths, entries}
	}, on_existing)
}

fn on_existing(mut p SftpPanel, r &core.TaskResult[Existing], mut cx core.ViewCx[SftpPanel]) {
	ex := r.value
	mut st := p.states[ex.key] or { return }
	colliding := collisions(ex.paths, ex.entries)
	if colliding.len == 0 {
		p.upload_batch(st, ex.paths, mut cx)
		return
	}
	others := keep_after(ex.paths, colliding, false).len > 0
	q := collision_prompt(colliding, others)
	p.ask(mut cx, q.title, q.body, q.replace, q.keep, true, core.listener_with[SftpPanel, CollisionAnswer, bool](cx.entity, CollisionAnswer{ex.key, ex.paths, colliding},
		on_collision_answer))
}

struct CollisionAnswer {
	key       u64
	paths     []string
	colliding []Collision
}

fn on_collision_answer(mut p SftpPanel, a CollisionAnswer, yes &bool, mut cx core.ViewCx[SftpPanel]) {
	mut st := p.states[a.key] or { return }
	keep := keep_after(a.paths, a.colliding, *yes)
	if keep.len > 0 {
		p.upload_batch(st, keep, mut cx)
	}
}

// upload_batch uploads sequentially; a failed or cancelled file does not stop the rest.
fn (mut p SftpPanel) upload_batch(st &Backend, paths []string, mut cx core.ViewCx[SftpPanel]) {
	core.spawn_task_with[SftpPanel, Job, BatchDone](mut cx, Job{
		client: st.client
		bus:    p.bus
		key:    st.key
		cwd:    st.cwd
		paths:  paths
	}, fn (j Job, _ &core.TaskCx) !BatchDone {
		mut client := j.client
		mut bus := j.bus
		for path in j.paths {
			name := upload_name(path)
			remote := ssh.remote_join(j.cwd, name)
			run_transfer(mut client, mut bus, j.key, .upload, name, path, remote)
		}
		return BatchDone{j.key}
	}, on_batch_done)
}

// run_transfer drives one transfer and reports its end on the bus (worker thread).
fn run_transfer(mut client Client, mut bus Bus, key u64, dir Direction, name string, local string, remote string) TransferStatus {
	id := bus.alloc_id()
	bus.post(BusMsg{
		key:       key
		kind:      .begin
		id:        id
		direction: dir
		name:      name
	})
	on_event := ssh.OnTransfer{
		ctx: prim.hold(&TransferPost{bus, key, id})
		f:   post_transfer_event
	}
	mut status := TransferStatus.completed
	mut msg := ''
	if dir == .upload {
		client.upload(id, local, remote, on_event) or {
			status = if is_cancelled_err(err) { .cancelled } else { .error }
			msg = if status == .error { err.msg() } else { '' }
		}
	} else {
		client.download(id, remote, local, on_event) or {
			status = if is_cancelled_err(err) { .cancelled } else { .error }
			msg = if status == .error { err.msg() } else { '' }
		}
	}
	bus.post(BusMsg{
		key:    key
		kind:   .finished
		id:     id
		status: status
		error:  msg
	})
	return status
}

// TransferPost is what a transfer's events are posted with (ssh.OnTransfer.ctx).
struct TransferPost {
	bus &Bus = unsafe { nil }
	key u64
	id  u64
}

fn post_transfer_event(ctx prim.Holder, e ssh.TransferEvent) {
	t := prim.held[TransferPost](ctx)
	mut bus := t.bus
	match e.kind {
		.progress {
			bus.post(BusMsg{
				key:      t.key
				kind:     .progress
				id:       t.id
				fraction: e.fraction
			})
		}
		.discovering {
			bus.post(BusMsg{
				key:   t.key
				kind:  .discovering
				id:    t.id
				files: e.files
			})
		}
		.cancelled {}
	}
}

fn on_batch_done(mut p SftpPanel, r &core.TaskResult[BatchDone], mut cx core.ViewCx[SftpPanel]) {
	if p.active != unsafe { nil } && p.active.key == r.value.key {
		p.refresh(mut cx)
	}
}

// do_download saves the selection: to the Local pane's folder when expanded, else through
// the Save dialog.
fn (mut p SftpPanel) do_download(mut cx core.ViewCx[SftpPanel]) {
	e := p.selected_entry() or {
		p.notice(.warning, 'Select a file or folder to download.', mut cx)
		return
	}
	if p.expanded {
		p.download_to_local(e, mut cx)
		return
	}
	got := cx.pick_files(platform.FileDialog{
		save:         true
		default_name: sanitize_file_name(e.name)
		dir:          os.home_dir()
	})
	if got.len > 0 {
		p.download_to(e, got[0], mut cx)
	}
}

// download_to_local downloads into the Local pane's folder, asking before replacing.
fn (mut p SftpPanel) download_to_local(e ssh.FileEntry, mut cx core.ViewCx[SftpPanel]) {
	dir := p.local.cwd
	if dir == '' {
		p.notice(.warning, 'The local pane has no directory open yet.', mut cx)
		return
	}
	// The name comes from the server: it must stay one component below the local folder.
	ssh.validate_remote_entry_name(e.name) or {
		p.notice(.error, '"${e.name}" cannot be saved locally: unsafe name.', mut cx)
		return
	}
	target := os.join_path(dir, e.name)
	if !os.exists(target) {
		p.download_to(e, target, mut cx)
		return
	}
	p.ask(mut cx, 'Replace Local File',
		'"${e.name}" already exists in the local folder. Replace it?', 'Replace', 'Cancel', true, core.listener_with[SftpPanel, LocalTarget, bool](cx.entity, LocalTarget{e, target}, fn (mut p SftpPanel, t LocalTarget, yes &bool, mut cx core.ViewCx[SftpPanel]) {
		if *yes {
			p.download_to(t.entry, t.path, mut cx)
		}
	}))
}

struct LocalTarget {
	entry ssh.FileEntry
	path  string
}

fn (mut p SftpPanel) download_to(e ssh.FileEntry, local string, mut cx core.ViewCx[SftpPanel]) {
	if p.active == unsafe { nil } {
		return
	}
	core.spawn_task_with[SftpPanel, Job, BatchDone](mut cx, Job{
		client: p.active.client
		bus:    p.bus
		key:    p.active.key
		name:   e.name
		path:   local
		remote: e.path
	}, fn (j Job, _ &core.TaskCx) !BatchDone {
		mut client := j.client
		mut bus := j.bus
		run_transfer(mut client, mut bus, j.key, .download, j.name, j.path, j.remote)
		return BatchDone{j.key}
	}, fn (mut p SftpPanel, _ &core.TaskResult[BatchDone], mut cx core.ViewCx[SftpPanel]) {
		if p.expanded {
			p.local_refresh(mut cx)
		}
	})
}
