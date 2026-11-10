// Directory listings, stat and the uid/gid names read from the remote host.
module ssh

// FileEntry is one remote file or folder: a listing row and the result of stat alike.
pub struct FileEntry {
pub:
	name        string
	path        string
	is_dir      bool
	is_symlink  bool
	size        u64
	modified    ?i64 // Unix seconds
	accessed    ?i64
	permissions u32
	uid         ?u32
	gid         ?u32
	owner       ?string
	group       ?string
}

const max_id_database_bytes = u64(4 * 1024 * 1024)

// read_bounded reads at most the cap of a remote file; the server is untrusted.
fn (mut s Sftp) read_bounded(path string) ![]u8 {
	handle := s.open_handle(path, fxf_read, Attrs{})!
	defer {
		s.close_handle(handle) or {}
	}
	mut out := []u8{}
	for u64(out.len) < max_id_database_bytes {
		mut w := new_writer(handle.len + 16)
		w.put_bytes(handle)
		w.put_u64(u64(out.len))
		w.put_u32(32768)
		body := s.call(fx_read, w.buf)!
		if body[0] == fx_status {
			status_error(body) or {
				if err is SftpError && err.status == .eof {
					break
				}
				return err
			}
			break
		}
		mut r := new_reader(body[5..])
		data := r.get_bytes()!
		if data.len == 0 {
			break
		}
		out << data
	}
	return out
}

// load_id_names reads /etc/passwd and /etc/group, best effort.
fn (mut s Sftp) load_id_names() {
	mut uids := map[u32]string{}
	mut gids := map[u32]string{}
	if data := s.read_bounded('/etc/passwd') {
		parse_passwd(data.bytestr(), mut uids, mut gids)
	}
	if data := s.read_bounded('/etc/group') {
		parse_group(data.bytestr(), mut gids)
	}
	s.mu.@lock()
	s.uid_names = uids.move()
	s.gid_names = gids.move()
	s.mu.unlock()
}

// parse_passwd: root:x:0:0:root:/root:/bin/bash; the user name also seeds the group names.
fn parse_passwd(text string, mut uids map[u32]string, mut gids map[u32]string) {
	for line in text.split_into_lines() {
		fields := line.split(':')
		if fields.len < 4 || !is_decimal(fields[2]) || !is_decimal(fields[3]) {
			continue
		}
		uids[fields[2].u32()] = fields[0]
		gid := fields[3].u32()
		if gid !in gids {
			gids[gid] = fields[0]
		}
	}
}

// parse_group: root:x:0: wins over the passwd fallback.
fn parse_group(text string, mut gids map[u32]string) {
	for line in text.split_into_lines() {
		fields := line.split(':')
		if fields.len >= 3 && is_decimal(fields[2]) {
			gids[fields[2].u32()] = fields[0]
		}
	}
}

fn is_decimal(s string) bool {
	if s == '' {
		return false
	}
	for c in s {
		if c < `0` || c > `9` {
			return false
		}
	}
	return true
}

fn (mut s Sftp) entry(name string, path string, a Attrs) FileEntry {
	s.mu.@lock()
	mut owner := ?string(none)
	mut group := ?string(none)
	if a.has(attr_uidgid) {
		if v := s.uid_names[a.uid] {
			owner = v
		}
		if v := s.gid_names[a.gid] {
			group = v
		}
	}
	s.mu.unlock()
	mut modified := ?i64(none)
	mut accessed := ?i64(none)
	if a.has(attr_times) {
		modified = i64(a.mtime)
		accessed = i64(a.atime)
	}
	mut uid := ?u32(none)
	mut gid := ?u32(none)
	if a.has(attr_uidgid) {
		uid = a.uid
		gid = a.gid
	}
	return FileEntry{
		name:        name
		path:        path
		is_dir:      a.is_dir()
		is_symlink:  a.is_symlink()
		size:        a.size
		modified:    modified
		accessed:    accessed
		permissions: a.perms
		uid:         uid
		gid:         gid
		owner:       owner
		group:       group
	}
}

// read_dir lists a folder: folders first, then files by case-insensitive name, no . or ...
pub fn (mut s Sftp) read_dir(path_ string) ![]FileEntry {
	mut path := remote_path(path_)
	if !path.starts_with('/') {
		if resolved := s.realpath(path) {
			path = resolved
		}
	}
	items := s.list(path)!
	mut entries := []FileEntry{cap: items.len}
	for item in items {
		if item.name == '.' || item.name == '..' {
			continue
		}
		entries << s.entry(item.name, remote_join(path, item.name), item.attrs)
	}
	entries.sort_with_compare(fn (a &FileEntry, b &FileEntry) int {
		if a.is_dir != b.is_dir {
			return if a.is_dir { -1 } else { 1 }
		}
		an, bn := a.name.to_lower(), b.name.to_lower()
		if an < bn {
			return -1
		}
		return if an > bn { 1 } else { 0 }
	})
	return entries
}

// stat returns the metadata of one path (follows symlinks, like stat(2)).
pub fn (mut s Sftp) stat(path_ string) !FileEntry {
	path := remote_path(path_)
	attrs := s.attrs_of(path, false)!
	return s.entry(remote_file_name(path), path, attrs)
}
