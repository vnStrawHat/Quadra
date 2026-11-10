// Temporary siblings and the atomic replace of a finished transfer.
module ssh

import os

fn (mut s Sftp) temp_name(base string, marker string) string {
	return '.${base}.oneterm-${os.getpid()}-${s.next_nonce()}.${marker}'
}

fn (mut s Sftp) temp_local(target string, marker string) string {
	return os.join_path(os.dir(target), s.temp_name(os.file_name(target), marker))
}

fn (mut s Sftp) temp_remote(target string, marker string) !string {
	name := target.all_after_last('/')
	if name == '' {
		return error('remote transfer target has no filename')
	}
	parent := if target.contains('/') { target.all_before_last('/') } else { '' }
	temp := s.temp_name(name, marker)
	if parent == '' && target.starts_with('/') {
		return '/${temp}'
	}
	if parent == '' {
		return temp
	}
	return '${parent}/${temp}'
}

// finalize_remote renames the temporary into place, keeping the old file until it worked.
fn (mut s Sftp) finalize_remote(temporary string, target string) ! {
	backup := s.temp_remote(target, 'backup')!
	mut had_target := false
	if attrs := s.attrs_of(target, true) {
		if attrs.is_dir() || attrs.is_symlink() {
			s.remove(temporary) or {}
			return error('refusing to replace a remote directory or symlink')
		}
		s.rename(target, backup) or {
			s.remove(temporary) or {}
			return err
		}
		had_target = true
	}
	s.rename(temporary, target) or {
		if had_target {
			s.rename(backup, target) or {}
		}
		s.remove(temporary) or {}
		return err
	}
	if had_target {
		s.remove(backup) or {}
	}
}

// finalize_local is the same dance on the local disk.
fn (mut s Sftp) finalize_local(temporary string, target string) ! {
	backup := s.temp_local(target, 'backup')
	mut had_target := false
	if os.exists(target) || os.is_link(target) {
		if os.is_link(target) || !os.is_file(target) {
			os.rm(temporary) or {}
			return error('refusing to replace a local directory or symlink')
		}
		os.mv(target, backup) or {
			os.rm(temporary) or {}
			return error('backup local target: ${err}')
		}
		had_target = true
	}
	os.mv(temporary, target) or {
		if had_target {
			os.mv(backup, target) or {}
		}
		os.rm(temporary) or {}
		return error('finalize local download: ${err}')
	}
	if had_target {
		os.rm(backup) or {}
	}
}
