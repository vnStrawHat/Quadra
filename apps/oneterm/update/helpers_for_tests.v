// Helpers shared by this module's tests; nothing else calls them. No test touches the network
// or GitHub: FakeClient serves canned responses, and every file stays under .tmp/.
module update

import compress.gzip
import encoding.binary
import hash.crc32
import os
import rand

const test_root = os.join_path(@VMODROOT, '.tmp', 'oneterm-tests', 'update')

fn test_dir(name string) string {
	dir := os.join_path(test_root, name, '${os.getpid()}-${rand.hex(8)}')
	os.rmdir_all(dir) or {} // rand repeats between runs: never inherit a leftover
	os.mkdir_all(dir) or { panic(err) }
	return dir
}

struct FakeClient {
mut:
	response      ReleaseResponse
	bytes         []u8
	last_etag     ?string
	fetches       int
	downloads     int
	fail_download bool
}

fn (mut f FakeClient) fetch_releases(repository string, etag ?string) !ReleaseResponse {
	f.fetches++
	f.last_etag = etag
	return f.response
}

fn (mut f FakeClient) download_to_file(url string, path string, max_bytes u64) ! {
	f.downloads++
	if f.fail_download {
		return error('network down')
	}
	if u64(f.bytes.len) > max_bytes {
		return download_too_large(u64(f.bytes.len), max_bytes)
	}
	os.write_file_array(path, f.bytes)!
}

const test_target = 'x86_64-pc-windows-msvc'
const valid_digest = 'sha256:${'a'.repeat(64)}'

fn asset(version string) GitHubAsset {
	return GitHubAsset{
		name:                 expected_asset_name(version, test_target)
		browser_download_url: 'https://example.invalid/${version}.zip'
		digest:               valid_digest
	}
}

fn release(tag string, prerelease bool, assets []GitHubAsset) GitHubRelease {
	return GitHubRelease{
		tag_name:   tag
		prerelease: prerelease
		html_url:   'https://example.invalid/${tag}'
		assets:     assets
	}
}

fn cached(version string, prerelease bool) UpdateCandidate {
	return UpdateCandidate{
		version:           version
		tag_name:          'v${version}'
		release_notes_url: 'https://example.invalid'
		asset_name:        expected_asset_name(version, test_target)
		asset_url:         'https://example.invalid/a.zip'
		asset_digest:      valid_digest
		target_triple:     test_target
		prerelease:        prerelease
	}
}

fn storage_in(dir string) UpdateStorage {
	return UpdateStorage{
		cache_dir:   os.join_path(dir, 'updates')
		config_path: os.join_path(dir, document_name)
	}
}

fn new_test_manager(dir string, config UpdateConfig, client &FakeClient) UpdateManager {
	return new_manager('owner/repo', config, client, '0.1.0', test_target, storage_in(dir), false)
}

// ZipEntry describes one member of a hand-built archive.
struct ZipEntry {
	name    string
	data    []u8
	symlink bool
}

// A stored (uncompressed) zip with the given members, built byte by byte.
fn build_zip(entries []ZipEntry) []u8 {
	mut out := []u8{}
	mut central := []u8{}
	for e in entries {
		offset := u32(out.len)
		crc := crc32.sum(e.data)
		mut local := []u8{len: 30}
		binary.little_endian_put_u32_at(mut local, 0x04034b50, 0)
		binary.little_endian_put_u16_at(mut local, 20, 4)
		binary.little_endian_put_u32_at(mut local, crc, 14)
		binary.little_endian_put_u32_at(mut local, u32(e.data.len), 18)
		binary.little_endian_put_u32_at(mut local, u32(e.data.len), 22)
		binary.little_endian_put_u16_at(mut local, u16(e.name.len), 26)
		out << local
		out << e.name.bytes()
		out << e.data
		mut rec := []u8{len: 46}
		binary.little_endian_put_u32_at(mut rec, 0x02014b50, 0)
		host := if e.symlink { u16(3) << 8 } else { u16(0) }
		binary.little_endian_put_u16_at(mut rec, host | 20, 4)
		binary.little_endian_put_u32_at(mut rec, crc, 16)
		binary.little_endian_put_u32_at(mut rec, u32(e.data.len), 20)
		binary.little_endian_put_u32_at(mut rec, u32(e.data.len), 24)
		binary.little_endian_put_u16_at(mut rec, u16(e.name.len), 28)
		if e.symlink {
			binary.little_endian_put_u32_at(mut rec, u32(0o120777) << 16, 38)
		}
		binary.little_endian_put_u32_at(mut rec, offset, 42)
		central << rec
		central << e.name.bytes()
	}
	start := u32(out.len)
	out << central
	mut eocd := []u8{len: 22}
	binary.little_endian_put_u32_at(mut eocd, 0x06054b50, 0)
	binary.little_endian_put_u16_at(mut eocd, u16(entries.len), 8)
	binary.little_endian_put_u16_at(mut eocd, u16(entries.len), 10)
	binary.little_endian_put_u32_at(mut eocd, u32(central.len), 12)
	binary.little_endian_put_u32_at(mut eocd, start, 16)
	out << eocd
	return out
}

// TarEntry describes one member of a hand-built tar.gz; `kind` is the tar type flag.
struct TarEntry {
	name string
	data []u8
	kind u8 = `0`
}

fn build_tar_gz(entries []TarEntry) []u8 {
	mut tar := []u8{}
	for e in entries {
		mut header := []u8{len: 512}
		for i, b in e.name.bytes() {
			header[i] = b
		}
		for i, b in '${e.data.len:011o}\0'.bytes() {
			header[124 + i] = b
		}
		header[156] = e.kind
		for i, b in 'ustar\x0000'.bytes() {
			header[257 + i] = b
		}
		for i in 148 .. 156 {
			header[i] = ` `
		}
		mut sum := 0
		for b in header {
			sum += b
		}
		for i, b in '${sum:06o}\0 '.bytes() {
			header[148 + i] = b
		}
		tar << header
		tar << e.data
		tar << []u8{len: (512 - e.data.len % 512) % 512}
	}
	tar << []u8{len: 1024}
	return gzip.compress(tar) or { panic(err) }
}

fn write_archive(dir string, name string, bytes []u8) string {
	path := os.join_path(dir, name)
	os.write_file_array(path, bytes) or { panic(err) }
	return path
}

fn extraction_error(archive string, destination string, limit u64) string {
	extract_archive_with_limit(archive, destination, limit) or { return err.msg() }
	panic('expected the extraction to fail')
}
