// Helpers for the live tests (interop*_test.v). Compiled out of the app: nothing else calls them.
module ssh

import io
import os
import sync
import quadra.platform
import quadra.prim

fn gate() ?(string, int) {
	if os.getenv('QUADRA_SSHD_TESTS') != '1' {
		return none
	}
	return os.getenv('SSHD_DIR'), os.getenv('SSHD_PORT').int()
}

fn key_auth(dir string, name string, pass string) AuthMethod {
	mut ka := KeyAuth{
		key_path: os.join_path(dir, name)
	}
	if pass != '' {
		ka.passphrase = secret(pass)
	}
	return AuthMethod(ka)
}

fn hop_for(_ string, port int, auth AuthMethod) Hop {
	return Hop{
		host:     '127.0.0.1'
		port:     port
		username: os.getenv('USERNAME')
		auth:     auth
	}
}

// known returns a known_hosts path holding every host key of the test server.
fn known(dir string, port int, tag string) string {
	path := os.join_path(dir, 'known_${tag}')
	mut text := ''
	for k in ['ed25519', 'ecdsa', 'rsa'] {
		pub_line := os.read_file(os.join_path(dir, 'hk_${k}.pub')) or { '' }
		fields := pub_line.fields()
		text += '[127.0.0.1]:${port} ${fields[0]} ${fields[1]}\n'
	}
	os.write_file(path, text) or { panic(err) }
	return path
}

fn base_config(dir string, port int, auth AuthMethod, tag string) Config {
	return Config{
		target:           hop_for(dir, port, auth)
		known_hosts_path: known(dir, port, tag)
		agent_pipe:       '\\\\.\\pipe\\no-such-agent-${tag}'
	}
}

// FuzzStream serves a fixed byte buffer, then io.Eof; writes go to sink when set (the fuzz tests).
@[heap]
struct FuzzStream {
mut:
	data []u8
	pos  int
	sink FuzzSink
}

interface FuzzSink {
mut:
	sent(wire []u8)
}

struct NoSink {}

fn (mut n NoSink) sent(wire []u8) {}

fn (mut s FuzzStream) read(mut buf []u8) !int {
	if s.pos >= s.data.len {
		return io.Eof{}
	}
	n := copy(mut buf, s.data[s.pos..])
	s.pos += n
	return n
}

fn (mut s FuzzStream) write(data []u8) ! {
	s.sink.sent(data)
}

fn (mut s FuzzStream) close() {}

// fuzz_ceiling fails a fuzz run whose heap passes 256 MB; called every input, it reads the heap
// every 100th.
fn fuzz_ceiling(i int) {
	if i % 100 == 99 {
		heap := gc_heap_usage().heap_size
		assert heap < 256 << 20, 'fuzz input ${i}: heap ${heap >> 20} MB'
	}
}

// plain_packets frames payloads as unencrypted packets.
fn plain_packets(payloads [][]u8) []u8 {
	mut out := []u8{}
	for p in payloads {
		out << build_packet(p, PlainSealer{})
	}
	return out
}

fn fuzz_transport(data []u8) &Transport {
	s := &FuzzStream{
		data: data
		sink: NoSink{}
	}
	return new_transport(Stream(s), Offer{}, HostCheck{})
}

// run_cmd opens an exec channel, returns (output, exit code).
fn run_cmd(mut conn Conn, cmd string) !(string, int) {
	mut ch := conn.open_channel('session', [], default_window)!
	mut w := new_writer(cmd.len + 8)
	w.put_str(cmd)
	if !ch.request('exec', w.buf, true)! {
		return error('exec refused')
	}
	mut out := []u8{}
	mut buf := []u8{len: 4096}
	for {
		n := ch.read(mut buf) or { break }
		out << buf[..n]
	}
	code := ch.wait_exit(2000)
	ch.close()
	return out.bytestr(), code
}

fn ptyopts(mut got ShellSink) platform.PtyOptions {
	return platform.PtyOptions{
		ctx:     prim.hold(got)
		on_data: fn (ctx prim.Holder, d []u8) {
			mut got := prim.held[ShellSink](ctx)
			got.mu.@lock()
			got.data << d
			got.mu.unlock()
		}
		on_exit: fn (ctx prim.Holder, code int) {
			mut got := prim.held[ShellSink](ctx)
			got.code = code
			got.done.post()
		}
	}
}

fn sftp_for(dir string, port int, tag string) !(&Session, &Sftp) {
	mut s := connect(base_config(dir, port, key_auth(dir, 'id_ed25519', ''), tag))!
	return s, s.sftp() or { return error('no SFTP') }
}

fn remote_area(tag string) string {
	dir := os.getenv('SSHD_DIR').replace('\\', '/')
	return '/${dir[0..1].to_lower()}${dir[2..]}/sftp/${tag}'
}

@[heap]
struct EventLog {
mut:
	events   []TransferEvent
	files    int
	warnings []string
}

@[heap]
struct ShellSink {
mut:
	mu   &sync.Mutex     = sync.new_mutex()
	done &sync.Semaphore = sync.new_semaphore()
	data []u8
	code int = -100
}

fn (mut s ShellSink) text() string {
	s.mu.@lock()
	defer {
		s.mu.unlock()
	}
	return s.data.bytestr()
}
