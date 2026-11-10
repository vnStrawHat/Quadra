// Remote mutations: open, rename, delete, new folder, properties. Each runs on a worker.
module sftpui

import ssh
import quadra.core
import quadra.kit

// OpDone is a finished mutation: the form closes on success, stays open on failure.
struct OpDone {
	key    u64
	ok     bool
	msg    string // success text
	prefix string // failure prefix
	err    string
	sftp   bool
	code   ssh.SftpStatus
}

fn describe_failure(d OpDone) string {
	if d.sftp {
		match d.code {
			.permission_denied {
				return '${d.prefix}: permission denied by the server.'
			}
			.no_such_file {
				return '${d.prefix}: the path no longer exists on the server. Refresh and try again.'
			}
			.failure {
				return '${d.prefix}: the server refused (${d.err}). The name may be in use or the folder not empty.'
			}
			else {}
		}
	}
	return '${d.prefix}: ${d.err}'
}

fn op_ok(key u64, msg string) OpDone {
	return OpDone{
		key: key
		ok:  true
		msg: msg
	}
}

fn op_error(key u64, prefix string, e IError) OpDone {
	if e is ssh.SftpError {
		return OpDone{
			key:    key
			prefix: prefix
			err:    e.msg()
			sftp:   true
			code:   e.status
		}
	}
	return OpDone{
		key:    key
		prefix: prefix
		err:    e.msg()
	}
}

fn (mut p SftpPanel) do_open(mut cx core.ViewCx[SftpPanel]) {
	e := p.selected_entry() or {
		p.notice(.warning, 'Select a file or folder to open.', mut cx)
		return
	}
	if e.is_dir {
		p.navigate_into(p.active.selected, mut cx)
	} else {
		p.do_download(mut cx)
	}
}

fn (mut p SftpPanel) do_rename(mut cx core.ViewCx[SftpPanel]) {
	e := p.selected_entry() or {
		p.notice(.warning, 'Select a file or folder to rename.', mut cx)
		return
	}
	p.form_op(.rename, e, e.name, mut cx)
}

fn (mut p SftpPanel) do_new_folder(mut cx core.ViewCx[SftpPanel]) {
	if p.active == unsafe { nil } {
		p.notice(.warning, 'No active SFTP connection is available.', mut cx)
		return
	}
	p.form_op(.new_folder, ssh.FileEntry{}, '', mut cx)
}

struct FormData {
	kind  SftpAction
	entry ssh.FileEntry
}

// form_op opens the rename or new-folder form; submit validates the name, then runs.
fn (mut p SftpPanel) form_op(kind SftpAction, e ssh.FileEntry, value string, mut cx core.ViewCx[SftpPanel]) {
	mut title, mut label, mut confirm := 'New Folder', 'Folder name', 'Create'
	if kind == .rename {
		title, label, confirm = 'Rename', 'New name', 'Rename'
	}
	p.form_modal = open_form(mut cx.Cx, title, label, value, label, confirm, core.listener_with[SftpPanel, FormData, string](cx.entity, FormData{kind, e},
		submit_form))
}

fn submit_form(mut p SftpPanel, d FormData, v &string, mut cx core.ViewCx[SftpPanel]) {
	name := validate_entry_name(*v) or {
		p.notice(.warning, err.msg(), mut cx)
		return
	}
	if p.active == unsafe { nil } {
		return
	}
	mut client := p.active.client
	key, cwd := p.active.key, p.active.cwd
	entry, is_rename := d.entry, d.kind == .rename
	core.spawn_task[SftpPanel, OpDone](mut cx, fn [mut client, key, cwd, name, entry, is_rename] (_ &core.TaskCx) !OpDone {
		if is_rename {
			target := ssh.remote_join(ssh.remote_parent(entry.path), name)
			client.rename(entry.path, target) or { return op_error(key, 'Rename failed', err) }
			return op_ok(key, 'Renamed to "${name}".')
		}
		client.mkdir(ssh.remote_join(cwd, name)) or {
			return op_error(key, 'Create folder failed', err)
		}
		return op_ok(key, 'Folder "${name}" created.')
	}, on_form_done)
}

fn on_form_done(mut p SftpPanel, r &core.TaskResult[OpDone], mut cx core.ViewCx[SftpPanel]) {
	d := r.value
	if !d.ok {
		p.notice(.error, describe_failure(d), mut cx)
		return
	}
	if p.form_modal != unsafe { nil } {
		kit.close_modal(mut cx.Cx, p.form_modal.id)
		p.form_modal = unsafe { nil }
	}
	p.notice(.success, d.msg, mut cx)
	if p.active != unsafe { nil } && p.active.key == d.key {
		p.refresh(mut cx)
	}
}

fn (mut p SftpPanel) do_delete(mut cx core.ViewCx[SftpPanel]) {
	e := p.selected_entry() or {
		p.notice(.warning, 'Select a file or folder to delete.', mut cx)
		return
	}
	p.ask(mut cx, 'Confirm Delete', delete_confirmation(e.name, e.is_dir), 'Delete', 'Cancel',
		true, core.listener_with[SftpPanel, ssh.FileEntry, bool](cx.entity, e, confirm_delete))
}

fn confirm_delete(mut p SftpPanel, e ssh.FileEntry, yes &bool, mut cx core.ViewCx[SftpPanel]) {
	if !*yes || p.active == unsafe { nil } {
		return
	}
	mut client := p.active.client
	key := p.active.key
	core.spawn_task[SftpPanel, OpDone](mut cx, fn [mut client, key, e] (_ &core.TaskCx) !OpDone {
		if e.is_dir {
			client.remove_dir_all(e.path) or { return op_error(key, 'Delete failed', err) }
		} else {
			client.remove(e.path) or { return op_error(key, 'Delete failed', err) }
		}
		return op_ok(key, 'Deleted successfully.')
	}, on_form_done)
}

struct Stated {
	key   u64
	ok    bool
	entry ssh.FileEntry
	err   string
}

fn (mut p SftpPanel) do_properties(mut cx core.ViewCx[SftpPanel]) {
	e := p.selected_entry() or {
		p.notice(.warning, 'Select a file or folder to view properties.', mut cx)
		return
	}
	mut client := p.active.client
	key := p.active.key
	path := e.path
	core.spawn_task[SftpPanel, Stated](mut cx, fn [mut client, key, path] (_ &core.TaskCx) !Stated {
		entry := client.stat(path) or { return Stated{
			key: key
			err: err.msg()
		} }
		return Stated{
			key:   key
			ok:    true
			entry: entry
		}
	}, fn (mut p SftpPanel, r &core.TaskResult[Stated], mut cx core.ViewCx[SftpPanel]) {
		p.bind(mut cx)
		if r.value.ok {
			show_properties(mut cx, r.value.entry)
		} else {
			p.notice(.error, 'Failed to get properties: ${r.value.err}', mut cx)
		}
	})
}
