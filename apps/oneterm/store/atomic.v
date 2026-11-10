// Atomic persistence for user-owned JSON documents (specs/25): a sibling lock, a same-directory
// temporary file, a `.bak` and a durable replacement. Domain modules own their schemas.
module store

import os
import os.filelock
import quadra.native.sys
import rand
import x.json2
import time

const lock_timeout = 10 * time.second

enum Sensitivity {
	ordinary    // .bak keeps the previous generation
	private_doc // holds sealed values but this write retires none: .bak keeps the previous generation
	secret      // .bak becomes a copy of the new document, so a retired value does not outlive the write
}

// A document that is not valid JSON, or not the expected shape.
pub struct InvalidData {
	Error
pub:
	reason string
}

pub fn (e InvalidData) msg() string {
	return e.reason
}

pub type JsonUpdate = fn (mut doc json2.Any) !

fn lock_path(path string) string {
	return os.join_path(os.dir(path), '.${os.file_name(path)}.lock')
}

fn temporary_path(path string) string {
	return os.join_path(os.dir(path), '.${os.file_name(path)}.tmp-${os.getpid()}-${rand.hex(8)}')
}

// backup_path is `vault.bak` for `vault.json`.
pub fn backup_path(path string) string {
	base := os.file_name(path)
	stem := if base.contains('.') { base.all_before_last('.') } else { base }
	return os.join_path(os.dir(path), stem + '.bak')
}

fn acquire_lock(path string) !filelock.FileLock {
	lp := lock_path(path)
	os.mkdir_all(os.dir(lp))!
	mut guard := filelock.new(lp)
	if !guard.wait_acquire(lock_timeout) {
		return error('timed out waiting for the lock of ${path}')
	}
	return guard
}

// atomic_write replaces the document whole; the last completed writer wins.
pub fn atomic_write(path string, bytes []u8) ! {
	mut guard := acquire_lock(path)!
	defer {
		guard.release()
	}
	atomic_write_unlocked(path, bytes, .ordinary)!
}

// atomic_write_sealed is atomic_write for a document holding sealed secrets: `retires_secret` is
// shown the document being replaced, and answers true when the new bytes drop or replace a sealed
// value, which makes `.bak` a copy of the new document.
pub fn atomic_write_sealed(path string, bytes []u8, retire fn (previous []u8) bool) ! {
	atomic_write_sealed_with(path, bytes, PlainRetire{retire}, fn (p PlainRetire, previous []u8) bool {
		return p.f(previous)
	})!
}

struct PlainRetire {
	f fn (previous []u8) bool = unsafe { nil }
}

// atomic_write_sealed_with is atomic_write_sealed for a test that needs data: it is an
// argument, not a capture (no capturing closures in apps, specs/14). D is a value type.
pub fn atomic_write_sealed_with[D](path string, bytes []u8, data D, retire fn (d D, previous []u8) bool) ! {
	mut guard := acquire_lock(path)!
	defer {
		guard.release()
	}
	sensitivity := if os.exists(path) && retire(data, os.read_bytes(path)!) {
		Sensitivity.secret
	} else {
		Sensitivity.private_doc
	}
	atomic_write_unlocked(path, bytes, sensitivity)!
}

fn atomic_write_unlocked(path string, bytes []u8, sensitivity Sensitivity) ! {
	os.mkdir_all(os.dir(path))!
	temporary := temporary_path(path)
	write_and_replace(path, temporary, bytes, sensitivity) or {
		os.rm(temporary) or {}
		return err
	}
}

fn write_and_replace(path string, temporary string, bytes []u8, sensitivity Sensitivity) ! {
	mut file := os.create(temporary)!
	file.write(bytes) or {
		file.close()
		return err
	}
	file.close()
	backup := backup_path(path)
	if os.is_dir(backup) { // os.cp would quietly copy into it
		return error('${backup} is a directory')
	}
	if sensitivity == .secret {
		os.cp(temporary, backup)!
	} else if os.exists(path) {
		os.cp(path, backup)!
	}
	sys.replace_file(temporary, path)!
}

// update_json_file is a read-modify-write transaction under the document's lock.
pub fn update_json_file(path string, update JsonUpdate) ! {
	update_json_file_as(path, PlainUpdate{update}, run_plain, .ordinary)!
}

// update_json_file_with is update_json_file for an update that needs data: it is an argument,
// not a capture (no capturing closures in apps, specs/14). D must be a value type.
pub fn update_json_file_with[D](path string, data D, update fn (d D, mut doc json2.Any) !) ! {
	update_json_file_as(path, data, update, .ordinary)!
}

struct PlainUpdate {
	f JsonUpdate = unsafe { nil }
}

fn run_plain(p PlainUpdate, mut doc json2.Any) ! {
	p.f(mut doc)!
}

// update_secret_json_file is the transaction for a document holding secret material: `.bak`
// always equals the new document.
pub fn update_secret_json_file(path string, update JsonUpdate) ! {
	update_json_file_as(path, PlainUpdate{update}, run_plain, .secret)!
}

// update_secret_json_file_with is update_secret_json_file with data (see update_json_file_with).
pub fn update_secret_json_file_with[D](path string, data D, update fn (d D, mut doc json2.Any) !) ! {
	update_json_file_as(path, data, update, .secret)!
}

fn update_json_file_as[D](path string, data D, update fn (d D, mut doc json2.Any) !, sensitivity Sensitivity) ! {
	mut guard := acquire_lock(path)!
	defer {
		guard.release()
	}
	mut doc := json2.Any(map[string]json2.Any{})
	if os.exists(path) {
		doc = parse_json(read_text(path)!) or { return InvalidData{
			reason: err.msg()
		} }
	}
	update(data, mut doc)!
	atomic_write_unlocked(path, encode_pretty(doc).bytes(), sensitivity)!
}

// read_text reads a whole document. A directory in the file's place is an error, like any
// other unreadable document, and never reads as "not found".
pub fn read_text(path string) !string {
	if os.is_dir(path) {
		return error('${path} is a directory')
	}
	return os.read_file(path)!
}

// remove_document removes a document and its `.bak` under its lock. Refused when `read_only`.
pub fn remove_document(path string, read_only bool) ! {
	if read_only {
		return error('a read-only window writes no configuration')
	}
	mut guard := acquire_lock(path)!
	defer {
		guard.release()
	}
	for file in [path, backup_path(path)] {
		if os.exists(file) {
			os.rm(file)!
		}
	}
}

// quarantine_file moves an unreadable document aside and returns its new path, empty when there
// is nothing to move or the window is read-only. It never overwrites an earlier quarantine.
pub fn quarantine_file(path string, read_only bool) !string {
	if read_only {
		return ''
	}
	mut guard := acquire_lock(path)!
	defer {
		guard.release()
	}
	if !os.exists(path) {
		return ''
	}
	name := os.file_name(path)
	for {
		target := os.join_path(os.dir(path),
			'.${name}.invalid-${os.getpid()}-${time.now().unix()}-${rand.hex(6)}')
		if os.exists(target) {
			continue
		}
		sys.replace_file(path, target)!
		return target
	}
	return ''
}
