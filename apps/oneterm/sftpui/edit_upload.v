// Upload of a saved working copy: conflict check, upload, new baseline.
module sftpui

import ssh
import quadra.core

struct Conflict {
	id       u64
	conflict bool
}

// is_conflict: anything but two equal, known timestamps cannot prove the remote is unchanged.
pub fn is_conflict(baseline ?i64, current ?i64) bool {
	b := baseline or { return true }
	c := current or { return true }
	return b != c
}

fn (mut p SftpPanel) begin_upload_edit(id u64, mut cx core.ViewCx[SftpPanel]) {
	mut s := p.edits[id] or { return }
	if s.uploading {
		s.pending = true
		return
	}
	s.uploading = true
	mut client := s.client
	remote, baseline := s.remote, s.baseline
	core.spawn_task[SftpPanel, Conflict](mut cx, fn [mut client, remote, baseline, id] (_ &core.TaskCx) !Conflict {
		e := client.stat(remote) or { ssh.FileEntry{} }
		return Conflict{id, is_conflict(baseline, e.modified)}
	}, on_conflict_checked)
}

fn on_conflict_checked(mut p SftpPanel, r &core.TaskResult[Conflict], mut cx core.ViewCx[SftpPanel]) {
	c := r.value
	s := p.edits[c.id] or { return }
	if c.conflict {
		name := ssh.remote_file_name(s.remote)
		p.ask(mut cx, 'Remote file changed',
			'The remote file "${name}" changed since you started editing. Uploading will overwrite those changes.',
			'OK', 'Cancel', false, core.listener_with[SftpPanel, u64, bool](cx.entity, c.id, fn (mut p SftpPanel, id u64, yes &bool, mut cx core.ViewCx[SftpPanel]) {
			if *yes {
				p.upload_edit(id, mut cx)
			} else {
				p.finish_upload(id, false, ?i64(none), mut cx)
			}
		}))
	} else {
		p.upload_edit(c.id, mut cx)
	}
}

struct EditUploaded {
	id    u64
	ok    bool
	mtime ?i64
}

fn (mut p SftpPanel) upload_edit(id u64, mut cx core.ViewCx[SftpPanel]) {
	mut s := p.edits[id] or { return }
	mut client := s.client
	mut bus := p.bus
	key, name, temp, remote := s.key, ssh.remote_file_name(s.remote), s.temp, s.remote
	core.spawn_task[SftpPanel, EditUploaded](mut cx, fn [mut client, mut bus, key, id, name, temp, remote] (_ &core.TaskCx) !EditUploaded {
		status := run_transfer(mut client, mut bus, key, .upload, name, temp, remote)
		ok := status == .completed
		mut mtime := ?i64(none)
		if ok {
			e := client.stat(remote) or { ssh.FileEntry{} }
			mtime = e.modified
		}
		return EditUploaded{id, ok, mtime}
	}, fn (mut p SftpPanel, r &core.TaskResult[EditUploaded], mut cx core.ViewCx[SftpPanel]) {
		p.finish_upload(r.value.id, r.value.ok, r.value.mtime, mut cx)
	})
}

// finish_upload settles an attempt: the baseline moves on success, the listing refreshes if
// the file is in the shown folder, and a save that arrived meanwhile runs now.
fn (mut p SftpPanel) finish_upload(id u64, ok bool, mtime ?i64, mut cx core.ViewCx[SftpPanel]) {
	mut s := p.edits[id] or { return }
	s.uploading = false
	if ok {
		s.baseline = mtime
		if p.active != unsafe { nil } && p.active.key == s.key
			&& p.active.cwd == ssh.remote_parent(s.remote) {
			p.refresh(mut cx)
		}
	}
	if s.pending {
		s.pending = false
		p.begin_upload_edit(id, mut cx)
	}
}
