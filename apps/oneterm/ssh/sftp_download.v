// Remote-to-local transfers: one file, or a folder listed in full first.
module ssh

import os

// download copies a remote file or folder to local. It blocks: call it on a worker thread.
// `id` names the transfer for cancel_transfer; a duplicate of a running id fails.
pub fn (mut s Sftp) download(id u64, remote_ string, local string, on_event OnTransfer) ! {
	mut cancel := s.register(id)!
	defer {
		s.unregister(id)
	}
	remote := remote_path(remote_)
	s.download_any(remote, local, on_event, mut cancel) or {
		if err is Cancelled {
			emit(on_event, .cancelled, 0, 0)
		}
		return err
	}
}

fn (mut s Sftp) download_any(remote string, local string, on_event OnTransfer, mut cancel Cancel) ! {
	attrs := s.attrs_of(remote, true)!
	if attrs.is_symlink() {
		return error('refusing to download a remote symlink')
	}
	if attrs.is_dir() {
		s.download_dir(remote, local, on_event, mut cancel)!
		return
	}
	total := attrs.size
	mut rep := Reporter{
		on_event: on_event
		total:    total
	}
	s.download_file(remote, attrs, local, mut cancel, mut rep)!
	emit(on_event, .progress, 1.0, 0)
}

// download_file writes a sibling .part file and moves it into place only when complete.
fn (mut s Sftp) download_file(remote string, attrs Attrs, local string, mut cancel Cancel, mut rep Reporter) ! {
	if os.is_link(local) {
		return error('refusing to overwrite a local symlink')
	}
	temporary := s.temp_local(local, 'part')
	handle := s.open_handle(remote, fxf_read, Attrs{})!
	mut out := os.create(temporary) or {
		s.close_handle_later(handle)
		return error('create local temporary file: ${err}')
	}
	s.pull(handle, mut out, attrs.size, mut cancel, mut rep) or {
		out.close()
		os.rm(temporary) or {}
		s.close_handle_later(handle)
		return err
	}
	out.close()
	s.close_handle(handle) or {}
	s.finalize_local(temporary, local) or {
		os.rm(temporary) or {}
		return err
	}
	apply_remote_times(local, attrs)
}

// apply_remote_times copies the permissions and times onto the local file, best effort.
fn apply_remote_times(local string, attrs Attrs) {
	if attrs.has(attr_times) {
		os.utime(local, attrs.atime, attrs.mtime) or {}
	}
	if attrs.has(attr_perm) && attrs.perms & 0o200 == 0 {
		// only the owner-write bit maps onto Windows
		os.chmod(local, 0o444) or {}
	}
}

struct RemoteFile {
	remote string
	local  string
	attrs  Attrs
}

// download_dir lists the whole tree first, for a true progress denominator.
fn (mut s Sftp) download_dir(remote string, local string, on_event OnTransfer, mut cancel Cancel) ! {
	os.mkdir_all(local) or { return error('create local dir: ${err}') }
	root := os.real_path(local)
	mut pending := [PendingCopy{remote, root, 0}]
	mut files := []RemoteFile{}
	mut total := u64(0)
	mut visited := 0
	for pending.len > 0 {
		if cancel.is_cancelled() {
			return Cancelled{}
		}
		cur := pending.pop()
		if cur.depth > max_traversal_depth {
			return error('remote download exceeded traversal depth limit')
		}
		for item in s.list(cur.remote)! {
			if cancel.is_cancelled() {
				return Cancelled{}
			}
			if item.name == '.' || item.name == '..' {
				continue
			}
			validate_remote_entry_name(item.name)!
			visited++
			if visited > max_traversal_entries {
				return error('remote download exceeded traversal entry limit')
			}
			if item.attrs.is_symlink() {
				return error('refusing to download remote symlink: ${item.name}')
			}
			child_remote := '${cur.remote.trim_right('/')}/${item.name}'
			child_local := safe_local_child(cur.local, item.name)!
			if item.attrs.is_dir() {
				create_safe_parent_dirs(root, os.join_path(child_local, '.placeholder'))!
				pending << PendingCopy{child_remote, child_local, cur.depth + 1}
			} else {
				total += item.attrs.size
				files << RemoteFile{child_remote, child_local, item.attrs}
			}
		}
		emit(on_event, .discovering, 0, files.len)
	}
	mut tree := &TreeProgress{}
	for f in files {
		create_safe_parent_dirs(root, f.local)!
		start := tree.done
		mut rep := Reporter{
			on_event: on_event
			total:    total
			start:    start
			tree:     tree
		}
		s.download_file(f.remote, f.attrs, f.local, mut cancel, mut rep)!
		tree.done = start + f.attrs.size
	}
	emit(on_event, .progress, 1.0, 0)
}

struct PendingCopy {
	remote string
	local  string
	depth  int
}
