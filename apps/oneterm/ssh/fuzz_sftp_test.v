// Fuzz: SFTP replies, read by the real client against a scripted server, and the sanitizing of
// remote names into local paths.
module ssh

import encoding.binary
import os
import quadra.testing.fuzz

// SftpScript is the server: each request the client sends is answered by the next reply.
@[heap]
struct SftpScript {
mut:
	ch      &Channel = unsafe { nil }
	replies [][]u8
	next    int
	keep_id bool // send the reply's own id (a mutated id must not hang the client)
}

fn (mut s SftpScript) sent(wire []u8) {
	plen := int(binary.big_endian_u32(wire))
	pad := int(wire[4])
	payload := wire[5..4 + plen - pad]
	if payload.len < 18 || payload[0] != msg_channel_data {
		return
	}
	if s.next >= s.replies.len {
		s.ch.got_eof()
		return
	}
	mut body := s.replies[s.next].clone()
	s.next++
	if body.len >= 5 && !s.keep_id {
		copy(mut body[1..5], payload[14..18])
	}
	mut frame := new_writer(body.len + 4)
	frame.put_bytes(body)
	s.ch.receive(frame.buf) or { s.ch.got_eof() }
	if s.keep_id {
		s.ch.got_eof()
	}
}

fn reply(kind u8, rest []u8) []u8 {
	mut w := new_writer(rest.len + 8)
	w.put_u8(kind)
	w.put_u32(0)
	w.put_raw(rest)
	return w.buf
}

fn full_attrs(mut w Writer) {
	put_attrs(mut w, Attrs{
		flags: attr_size | attr_uidgid | attr_perm | attr_times
		size:  1234
		perms: mode_dir | 0o755
	})
}

fn sftp_corpus() [][]u8 {
	mut handle := new_writer(16)
	handle.put_str('h1')
	mut name := new_writer(128)
	name.put_u32(2)
	for n in ['a.txt', 'sub'] {
		name.put_str(n)
		name.put_str('-rw-r--r-- 1 u g 0 Jan 1 ${n}')
		full_attrs(mut name)
	}
	mut one := new_writer(64)
	one.put_u32(1)
	one.put_str('/home/u')
	one.put_str('')
	one.put_u32(0)
	mut eof := new_writer(16)
	eof.put_u32(1)
	eof.put_str('eof')
	eof.put_str('')
	mut attrs := new_writer(64)
	full_attrs(mut attrs)
	mut ext := new_writer(64)
	ext.put_u32(attr_extended)
	ext.put_u32(1)
	ext.put_str('k')
	ext.put_str('v')
	mut data := new_writer(64)
	data.put_str('root:x:0:0::/root:/bin/sh\nu:x:1000:1000:::\n')
	mut limits := new_writer(40)
	for v in [u64(1) << 15, 1 << 15, 1 << 15, 64] {
		limits.put_u64(v)
	}
	return [reply(fx_handle, handle.buf), reply(fx_name, name.buf),
		reply(fx_status, eof.buf), reply(fx_status, [u8(0), 0, 0, 0]),
		reply(fx_attrs, attrs.buf), reply(fx_attrs, ext.buf),
		reply(fx_name, one.buf), reply(fx_data, data.buf), reply(fx_extended_reply, limits.buf)]
}

// The replies of a well-behaved server for the operations below, in order.
const sftp_flow = [0, 1, 2, 3, 4, 6, 0, 7, 2, 3, 8]

// run_sftp_script runs the operations against the replies; returns the listing size, or -1.
fn run_sftp_script(replies [][]u8, keep_id bool) int {
	mut c := new_conn(fuzz_transport([]))
	mut ch := new_channel(c, 0, 4 * 1024 * 1024)
	ch.remote_window = 1 << 30
	mut script := &SftpScript{
		ch:      ch
		replies: replies
		keep_id: keep_id
	}
	c.tr.stream = Stream(&FuzzStream{
		sink: script
	})
	mut s := &Sftp{
		ch: ch
	}
	spawn s.demux()
	listed := s.read_dir('/r') or { []FileEntry{len: 0} }
	s.stat('/r/a.txt') or {}
	s.realpath('x') or {}
	s.read_bounded('/etc/passwd') or {}
	s.query_limits()
	ch.got_eof()
	return if listed.len > 0 { listed.len } else { -1 }
}

fn test_script_reaches_the_decoders() {
	corpus := sftp_corpus()
	assert run_sftp_script(sftp_flow.map(corpus[it]), false) == 2
}

fn test_short_version_reply_is_an_error() {
	// new_sftp sliced body[5..] of a VERSION reply of any length
	for body in [[fx_version], [fx_version, 0, 0], [fx_version, 0, 0, 0]] {
		if _ := parse_version(body) {
			assert false
		}
	}
	mut v := new_writer(64)
	v.put_u8(fx_version)
	v.put_u32(3)
	v.put_str('limits@openssh.com')
	v.put_str('1')
	assert parse_version(v.buf) or { panic(err) }
}

fn test_fuzz_sftp_replies() {
	n := fuzz.iters(150)
	mut rng := fuzz.new_rng(0x5f7)
	corpus := sftp_corpus()
	mut w := fuzz.watch('ssh sftp replies', n)
	for i in 0 .. n {
		mut replies := [][]u8{}
		for k in sftp_flow {
			replies << if rng.chance(30) {
				rng.mutate(corpus[k], corpus)
			} else {
				corpus[k]
			}
		}
		mut size := 0
		for r in replies {
			size += r.len
		}
		w.start()
		parse_version(rng.mutate([fx_version, 0, 0, 0, 3], corpus)) or {}
		_ = run_sftp_script(replies, rng.chance(5))
		w.done(i, size, 2000)
		fuzz_ceiling(i)
	}
	w.finish(64)
}

fn test_fuzz_remote_names() {
	n := fuzz.iters(3000)
	mut rng := fuzz.new_rng(0x9a7)
	root := os.norm_path(os.join_path(@DIR, '..', '..', '..', '.tmp', 'ssh-unit', 'dl'))
	corpus := ['a.txt', '..', '../x', r'..\x', 'C:\\x', 'c:x', 'COM1', 'nul.txt', 'a/b',
		r'\\srv\share', 'x\0y', '  ', '.hidden', 'tail.', '漢字.md', '/abs/path', '//a//b//',
		r'a\\b'].map(it.bytes())
	mut w := fuzz.watch('ssh remote names', n)
	for i in 0 .. n {
		name := rng.mutate(corpus[rng.intn(corpus.len)], corpus).bytestr()
		w.start()
		p := remote_path(name)
		assert !p.contains('//') && !p.contains('\\'), name
		assert remote_path(p) == p, name
		remote_join('/base', name)
		remote_parent(p)
		remote_file_name(p)
		if local := safe_local_child(root, name) {
			assert os.dir(local) == root, name
			assert os.file_name(local) == name, name
			assert below(root, local), name
		}
		w.done(i, name.len, 2000)
		fuzz_ceiling(i)
	}
	w.finish(64)
}
