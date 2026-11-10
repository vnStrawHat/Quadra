module update

import compress.deflate
import encoding.binary
import hash.crc32
import os

fn dest(dir string) string {
	out := os.join_path(dir, 'out')
	os.mkdir_all(out) or { panic(err) }
	return out
}

fn zip_of(dir string, entries []ZipEntry) string {
	return write_archive(dir, 'a.zip', build_zip(entries))
}

fn file_entry(name string, data string) ZipEntry {
	return ZipEntry{
		name: name
		data: data.bytes()
	}
}

fn test_zip_with_nested_dist_layout_extracts_and_validates() {
	dir := test_dir('zip-nested')
	archive := zip_of(dir, [
		file_entry('dist/oneterm-0.2.0-${test_target}/oneterm.exe', 'exe'),
		file_entry('dist/oneterm-0.2.0-${test_target}/x64/OpenConsole.exe', 'con'),
		ZipEntry{
			name: 'dist/oneterm-0.2.0-${test_target}/empty/'
		},
	])
	out := dest(dir)
	extract_archive(archive, out)!
	package := validate_staged_package(out, test_target)!
	assert os.read_file(os.join_path(package, 'oneterm.exe'))! == 'exe'
	assert os.read_file(os.join_path(package, 'x64', 'OpenConsole.exe'))! == 'con'
	assert os.is_dir(os.join_path(package, 'empty'))
}

fn test_a_deflated_zip_member_is_verified_and_extracted() {
	dir := test_dir('zip-deflate')
	original := 'hello hello hello hello hello'.bytes()
	compressed := deflate.compress_raw(original)!
	mut zip := build_zip([ZipEntry{
		name: 'oneterm.exe'
		data: compressed
	}])
	// The local header of the first member is at 0, the central directory right after the data.
	central := 30 + 'oneterm.exe'.len + compressed.len
	binary.little_endian_put_u16_at(mut zip, 8, 8) // local method
	binary.little_endian_put_u16_at(mut zip, 8, central + 10) // central method
	binary.little_endian_put_u32_at(mut zip, crc32.sum(original), central + 16)
	binary.little_endian_put_u32_at(mut zip, u32(original.len), central + 24)
	archive := write_archive(dir, 'a.zip', zip)
	out := dest(dir)
	extract_archive(archive, out)!
	assert os.read_bytes(os.join_path(out, 'oneterm.exe'))! == original
}

fn test_zip_slip_entry_is_rejected_before_anything_escapes() {
	dir := test_dir('zip-slip')
	out := dest(dir)
	message := extraction_error(zip_of(dir, [file_entry('../escape.txt', 'x')]), out, 1024)
	assert message.contains('unsafe path')
	assert !os.exists(os.join_path(dir, 'escape.txt'))
}

fn test_zip_absolute_drive_and_backslash_rooted_entries_are_rejected() {
	for name in ['/etc/passwd', 'C:\\x.txt', 'C:/x.txt', '\\root.txt', 'a/../../b'] {
		dir := test_dir('zip-root')
		message := extraction_error(zip_of(dir, [file_entry(name, 'x')]), dest(dir), 1024)
		assert message.contains('unsafe path'), name
	}
}

fn test_zip_symlink_entry_is_rejected() {
	dir := test_dir('zip-link')
	archive := zip_of(dir, [
		ZipEntry{
			name:    'link'
			data:    'target'.bytes()
			symlink: true
		},
	])
	message := extraction_error(archive, dest(dir), 1024)
	assert message.contains('symlink')
}

fn test_zip_expansion_beyond_budget_is_rejected() {
	dir := test_dir('zip-budget')
	archive := zip_of(dir, [file_entry('big', 'x'.repeat(4096))])
	assert extraction_error(archive, dest(dir), 1024).contains('expands beyond')
}

fn test_corrupt_zip_is_rejected() {
	dir := test_dir('zip-corrupt')
	archive := write_archive(dir, 'a.zip', 'not a zip file'.bytes())
	assert extraction_error(archive, dest(dir), 1024).contains('invalid zip')
}

fn test_tar_with_nested_layout_extracts_and_validates() {
	dir := test_dir('tar-nested')
	target := 'x86_64-unknown-linux-gnu'
	archive := write_archive(dir, 'a.tar.gz', build_tar_gz([
		TarEntry{
			name: 'dist/oneterm-0.2.0-${target}/'
			kind: `5`
		},
		TarEntry{
			name: 'dist/oneterm-0.2.0-${target}/oneterm'
			data: 'bin'.bytes()
		},
	]))
	out := dest(dir)
	extract_archive(archive, out)!
	package := validate_staged_package(out, target)!
	assert os.read_file(os.join_path(package, 'oneterm'))! == 'bin'
}

fn test_tar_parent_directory_entry_is_rejected() {
	dir := test_dir('tar-parent')
	archive := write_archive(dir, 'a.tar.gz', build_tar_gz([
		TarEntry{
			name: '../escape'
			data: 'x'.bytes()
		},
	]))
	assert extraction_error(archive, dest(dir), 4096).contains('unsafe path')
}

fn test_tar_symlink_entry_is_rejected() {
	dir := test_dir('tar-link')
	archive := write_archive(dir, 'a.tar.gz', build_tar_gz([
		TarEntry{
			name: 'link'
			kind: `2`
		},
	]))
	assert extraction_error(archive, dest(dir), 4096).contains('links')
}

fn test_tar_expansion_beyond_budget_is_rejected() {
	dir := test_dir('tar-budget')
	archive := write_archive(dir, 'a.tar.gz', build_tar_gz([
		TarEntry{
			name: 'big'
			data: 'x'.repeat(4096).bytes()
		},
	]))
	assert extraction_error(archive, dest(dir), 1024).contains('expands beyond')
}

fn test_unsupported_archive_extension_is_rejected() {
	dir := test_dir('unsupported')
	archive := write_archive(dir, 'a.rar', 'x'.bytes())
	assert extraction_error(archive, dest(dir), 1024).contains('unsupported')
}

fn test_a_package_without_the_required_file_is_refused() {
	dir := test_dir('missing-exe')
	out := dest(dir)
	validate_staged_package(out, test_target) or {
		assert err.msg().contains('missing required')
		return
	}
	assert false
}
