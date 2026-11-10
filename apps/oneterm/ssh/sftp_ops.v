// SFTP metadata operations and bounded recursive deletion.
module ssh

const max_traversal_depth = 64
const max_traversal_entries = 100_000

// open_handle opens a remote file, honouring limits@openssh.com on open handles.
fn (mut s Sftp) open_handle(path string, flags u32, attrs Attrs) ![]u8 {
	s.mu.@lock()
	if s.max_open > 0 && s.open_now >= s.max_open {
		s.mu.unlock()
		return error('too many open SFTP handles')
	}
	s.open_now++
	s.mu.unlock()
	mut w := new_writer(path.len + 32)
	w.put_str(path)
	w.put_u32(flags)
	put_attrs(mut w, attrs)
	body := s.call(fx_open, w.buf) or {
		s.handle_released()
		return err
	}
	return s.expect_handle(body)
}

fn (mut s Sftp) handle_released() {
	s.mu.@lock()
	if s.open_now > 0 {
		s.open_now--
	}
	s.mu.unlock()
}

fn (mut s Sftp) expect_handle(body []u8) ![]u8 {
	if body[0] == fx_status {
		s.handle_released()
		status_error(body)!
		return error('SFTP open failed')
	}
	if body[0] != fx_handle {
		s.handle_released()
		return error('unexpected SFTP reply ${body[0]}')
	}
	mut r := new_reader(body[5..])
	return r.get_bytes()!.clone()
}

// close_handle closes and waits, so a deferred server error is seen.
fn (mut s Sftp) close_handle(handle []u8) ! {
	mut w := new_writer(handle.len + 8)
	w.put_bytes(handle)
	defer {
		s.handle_released()
	}
	s.call_status(fx_close, w.buf)!
}

// close_handle_later releases a handle without blocking the caller (after cancel).
fn (mut s Sftp) close_handle_later(handle []u8) {
	spawn close_handle_quietly(mut s, handle)
}

fn close_handle_quietly(mut s Sftp, handle []u8) {
	s.close_handle(handle) or {}
}

// attrs_of returns the attributes of a path; lstat does not follow symlinks.
fn (mut s Sftp) attrs_of(path string, lstat bool) !Attrs {
	kind := if lstat { fx_lstat } else { fx_stat }
	body := s.call(kind, path_payload(path))!
	if body[0] == fx_status {
		status_error(body)!
		return error('SFTP stat failed')
	}
	if body[0] != fx_attrs {
		return error('unexpected SFTP reply ${body[0]}')
	}
	mut r := new_reader(body[5..])
	return parse_attrs(mut r)
}

fn (mut s Sftp) set_attrs(path string, attrs Attrs) ! {
	mut w := new_writer(path.len + 32)
	w.put_str(path)
	put_attrs(mut w, attrs)
	s.call_status(fx_setstat, w.buf)!
}

// realpath resolves a path on the server.
pub fn (mut s Sftp) realpath(path string) !string {
	body := s.call(fx_realpath, path_payload(remote_path(path)))!
	if body[0] == fx_status {
		status_error(body)!
		return error('SFTP realpath failed')
	}
	mut r := new_reader(body[5..])
	if body[0] != fx_name || r.get_u32()! < 1 {
		return error('unexpected SFTP reply ${body[0]}')
	}
	return remote_path(r.get_str()!)
}

// rename renames a file or folder. Plain SFTP rename: it fails if the target exists.
pub fn (mut s Sftp) rename(from string, to string) ! {
	mut w := new_writer(from.len + to.len + 8)
	w.put_str(remote_path(from))
	w.put_str(remote_path(to))
	s.call_status(fx_rename, w.buf)!
}

// remove deletes one file.
pub fn (mut s Sftp) remove(path string) ! {
	s.call_status(fx_remove, path_payload(remote_path(path)))!
}

// mkdir creates one folder.
pub fn (mut s Sftp) mkdir(path string) ! {
	mut w := new_writer(path.len + 16)
	w.put_str(remote_path(path))
	w.put_u32(0)
	s.call_status(fx_mkdir, w.buf)!
}

fn (mut s Sftp) rmdir(path string) ! {
	s.call_status(fx_rmdir, path_payload(path))!
}

// DirItem is one raw directory entry.
struct DirItem {
	name  string
	attrs Attrs
}

// list reads a whole directory with its own handle.
fn (mut s Sftp) list(path string) ![]DirItem {
	body := s.call(fx_opendir, path_payload(path))!
	handle := s.expect_handle_counted(body)!
	defer {
		s.close_handle(handle) or {}
	}
	mut out := []DirItem{}
	mut w := new_writer(handle.len + 8)
	w.put_bytes(handle)
	for {
		reply := s.call(fx_readdir, w.buf)!
		if reply[0] == fx_status {
			status_error(reply) or {
				if err is SftpError && err.status == .eof {
					break
				}
				return err
			}
			break
		}
		if reply[0] != fx_name {
			return error('unexpected SFTP reply ${reply[0]}')
		}
		mut r := new_reader(reply[5..])
		count := r.get_u32()!
		for _ in 0 .. count {
			name := r.get_str()!
			r.get_str()! // long name
			attrs := parse_attrs(mut r)!
			out << DirItem{name, attrs}
		}
	}
	return out
}

// expect_handle_counted is expect_handle for handles not counted before the request.
fn (mut s Sftp) expect_handle_counted(body []u8) ![]u8 {
	s.mu.@lock()
	s.open_now++
	s.mu.unlock()
	return s.expect_handle(body)
}

// remove_dir_all removes a tree. A symlink or a non-directory root is unlinked, and symlinks
// below the root are unlinked, never followed.
pub fn (mut s Sftp) remove_dir_all(path_ string) ! {
	path := remote_path(path_)
	attrs := s.attrs_of(path, true)!
	if attrs.is_symlink() || !attrs.is_dir() {
		s.call_status(fx_remove, path_payload(path))!
		return
	}
	mut pending := [PendingDir{path, false, 0}]
	mut visited := 0
	for pending.len > 0 {
		current := pending.pop()
		if current.depth > max_traversal_depth {
			return error('remote deletion exceeded traversal depth limit')
		}
		if current.expanded {
			s.rmdir(current.path)!
			continue
		}
		items := s.list(current.path)!
		pending << PendingDir{current.path, true, current.depth}
		for item in items {
			if item.name == '.' || item.name == '..' {
				continue
			}
			validate_remote_entry_name(item.name)!
			visited++
			if visited > max_traversal_entries {
				return error('remote deletion exceeded traversal entry limit')
			}
			child := '${current.path.trim_right('/')}/${item.name}'
			if item.attrs.is_dir() && !item.attrs.is_symlink() {
				pending << PendingDir{child, false, current.depth + 1}
			} else {
				s.call_status(fx_remove, path_payload(child))!
			}
		}
	}
}

struct PendingDir {
	path     string
	expanded bool
	depth    int
}
