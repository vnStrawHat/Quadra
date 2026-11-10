// Release archive extraction: zip and tar.gz, refusing anything that could escape the staging
// directory or fill the disk (specs/25).
module update

import compress.gzip
import encoding.binary
import os

// Upper bound for the bytes an update archive may expand to.
pub const max_extracted_bytes = u64(2) * 1024 * 1024 * 1024

// Charged per entry; `abort` is set by a chunk callback that goes over.
@[heap]
struct Sink {
mut:
	file      os.File
	buffer    []u8 // used instead of file when `to_buffer`
	remaining u64
	limit     u64
	to_buffer bool
	failed    bool
	failure   string
}

fn (s &Sink) exceeded() IError {
	return error('update archive expands beyond the ${s.limit} byte limit')
}

fn sink_chunk(chunk []u8, userdata voidptr) int {
	mut s := unsafe { &Sink(userdata) }
	if u64(chunk.len) > s.remaining {
		s.failed = true
		s.failure = s.exceeded().msg()
		return 0
	}
	s.remaining -= u64(chunk.len)
	if s.to_buffer {
		s.buffer << chunk
	} else {
		s.file.write(chunk) or {
			s.failed = true
			s.failure = err.msg()
			return 0
		}
	}
	return chunk.len
}

pub fn extract_archive(archive_path string, destination string) ! {
	extract_archive_with_limit(archive_path, destination, max_extracted_bytes)!
}

fn extract_archive_with_limit(archive_path string, destination string, limit u64) ! {
	name := os.file_name(archive_path)
	if name.ends_with('.zip') {
		extract_zip(archive_path, destination, limit)!
	} else if name.ends_with('.tar.gz') || name.ends_with('.tgz') {
		extract_tar_gz(archive_path, destination, limit)!
	} else {
		return error('unsupported update archive format: ${name}')
	}
}

fn reject_unsafe_path(path string) ! {
	if path.starts_with('/') || path.starts_with('\\')
		|| (path.len >= 2 && path[1] == `:`) || path.contains('\0') {
		return error('update archive contains unsafe path: ${path}')
	}
	for segment in path.replace('\\', '/').split('/') {
		if segment == '..' || (segment.len >= 2 && segment[1] == `:`) {
			return error('update archive contains unsafe path: ${path}')
		}
	}
}

// A hand-rolled central directory reader, not compress.szip: that exposes neither external
// attributes (symlinks) nor an expansion budget.
fn extract_zip(archive_path string, destination string, limit u64) ! {
	data := os.read_bytes(archive_path)!
	mut eocd := -1
	for i := data.len - 22; i >= 0 && i >= data.len - 65557; i-- {
		if binary.little_endian_u32_at(data, i) == 0x06054b50 {
			eocd = i
			break
		}
	}
	if eocd < 0 {
		return error('invalid zip archive: no end of central directory')
	}
	entries := binary.little_endian_u16_at(data, eocd + 10)
	mut pos := int(binary.little_endian_u32_at(data, eocd + 16))
	mut remaining := limit
	for _ in 0 .. entries {
		if pos + 46 > data.len || binary.little_endian_u32_at(data, pos) != 0x02014b50 {
			return error('invalid zip entry: bad central directory')
		}
		made_by := binary.little_endian_u16_at(data, pos + 4)
		method := binary.little_endian_u16_at(data, pos + 10)
		crc := binary.little_endian_u32_at(data, pos + 16)
		packed := binary.little_endian_u32_at(data, pos + 20)
		declared := binary.little_endian_u32_at(data, pos + 24)
		name_len := int(binary.little_endian_u16_at(data, pos + 28))
		extra_len := int(binary.little_endian_u16_at(data, pos + 30))
		comment_len := int(binary.little_endian_u16_at(data, pos + 32))
		external := binary.little_endian_u32_at(data, pos + 38)
		local := int(binary.little_endian_u32_at(data, pos + 42))
		if pos + 46 + name_len > data.len {
			return error('invalid zip entry: name past the end of the archive')
		}
		name := data[pos + 46..pos + 46 + name_len].bytestr()
		pos += 46 + name_len + extra_len + comment_len
		reject_unsafe_path(name) or { return error('unsafe path in zip archive: ${name}') }
		if (made_by >> 8) == 3 && (external >> 16) & 0o170000 == 0o120000 {
			return error('update archive contains a symlink, which is not allowed: ${name}')
		}
		output := os.join_path_single(destination, name.replace('\\', '/'))
		if name.ends_with('/') {
			os.mkdir_all(output)!
			continue
		}
		os.mkdir_all(os.dir(output))!
		if u64(declared) > remaining {
			return error('update archive expands beyond the ${limit} byte limit')
		}
		if local + 30 > data.len || binary.little_endian_u32_at(data, local) != 0x04034b50 {
			return error('invalid zip entry: bad local header')
		}
		start := local + 30 + int(binary.little_endian_u16_at(data, local + 26)) +
			int(binary.little_endian_u16_at(data, local + 28))
		if start + int(packed) > data.len {
			return error('invalid zip entry: data past the end of the archive')
		}
		remaining = write_zip_member(data[start..start + int(packed)], method, crc, declared,
			output, remaining, limit)!
	}
}

// Deflated members go through the gzip framing: the callback inflater recognises a raw stream
// only by its first bytes, and the zip already carries the CRC-32 and size gzip wants.
fn write_zip_member(payload []u8, method u16, crc u32, size u32, output string, remaining u64, limit u64) !u64 {
	mut sink := &Sink{
		file:      os.create(output)!
		remaining: remaining
		limit:     limit
	}
	defer {
		sink.file.close()
	}
	match method {
		0 {
			sink_chunk(payload, voidptr(sink))
		}
		8 {
			mut framed := [u8(0x1f), 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff]
			framed << payload
			mut trailer := []u8{len: 8}
			binary.little_endian_put_u32_at(mut trailer, crc, 0)
			binary.little_endian_put_u32_at(mut trailer, size, 4)
			framed << trailer
			gzip.decompress_with_callback(framed, sink_chunk, voidptr(sink))!
		}
		else {
			return error('invalid zip entry: unsupported compression method ${method}')
		}
	}

	if sink.failed {
		return error(sink.failure)
	}
	return sink.remaining
}

fn parse_octal(field []u8) !u64 {
	text := field.bytestr().trim('\0 ')
	if text == '' {
		return 0
	}
	if !text.contains_only('01234567') {
		return error('invalid tar entry: bad numeric field')
	}
	return text.parse_uint(8, 64)!
}

fn extract_tar_gz(archive_path string, destination string, limit u64) ! {
	// Slack over the budget for the tar headers themselves.
	mut sink := &Sink{
		remaining: limit + 1024 * 1024
		limit:     limit
		to_buffer: true
	}
	gzip.decompress_with_callback(os.read_bytes(archive_path)!, sink_chunk, voidptr(sink)) or {
		return error('invalid tar archive: ${err.msg()}')
	}
	if sink.failed {
		return error(sink.failure)
	}
	data := sink.buffer
	mut pos := 0
	mut remaining := limit
	mut long_name := ''
	for pos + 512 <= data.len {
		block := data[pos..pos + 512]
		if block.all(it == 0) {
			break
		}
		mut sum := u64(0)
		for i, b in block {
			sum += if i >= 148 && i < 156 { u64(` `) } else { u64(b) }
		}
		if parse_octal(block[148..156])! != sum {
			return error('invalid tar entry: checksum mismatch')
		}
		size := int(parse_octal(block[124..136])!)
		kind := block[156]
		body := pos + 512
		next := body + ((size + 511) / 512) * 512
		if next > data.len {
			return error('invalid tar entry: data past the end of the archive')
		}
		pos = next
		if kind in [`L`, `K`, `x`, `g`] {
			if kind == `L` {
				long_name = data[body..body + size].bytestr().trim('\0')
			}
			continue
		}
		if kind in [`1`, `2`] {
			return error('update archive contains links, which are not allowed')
		}
		if kind !in [`0`, 0, `5`] {
			return error('update archive contains an unsupported entry type')
		}
		mut name := if long_name != '' { long_name } else { block[..100].bytestr().trim('\0') }
		long_name = ''
		if block[257..262].bytestr() == 'ustar' {
			prefix := block[345..500].bytestr().trim('\0')
			if prefix != '' {
				name = '${prefix}/${name}'
			}
		}
		reject_unsafe_path(name)!
		output := os.join_path_single(destination, name)
		if kind == `5` || name.ends_with('/') {
			os.mkdir_all(output)!
			continue
		}
		if u64(size) > remaining {
			return error('update archive expands beyond the ${limit} byte limit')
		}
		remaining -= u64(size)
		os.mkdir_all(os.dir(output))!
		mut f := os.create(output)!
		f.write(data[body..body + size])!
		f.close()
	}
}
