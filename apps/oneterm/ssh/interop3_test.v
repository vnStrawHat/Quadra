// Live agent tests: the agent protocol against sshd, through a fake agent on a test pipe.
module ssh

import os
import time
import quadra.native.sys
import quadra.platform.win

type FnCreateNamedPipeW = fn (name &u16, open_mode u32, pipe_mode u32, max_instances u32, out_size u32, in_size u32, timeout u32, sec voidptr) voidptr

type FnConnectNamedPipe = fn (h voidptr, ov voidptr) int

type FnDisconnectNamedPipe = fn (h voidptr) int

const test_pipe = r'\\.\pipe\quadra-ssh-test-agent'

// fake_agent serves one pipe instance per client, signing with one ed25519 key.
fn fake_agent(key PrivateKey) {
	lib := win.load_system_dll('kernel32.dll') or { panic(err) }
	create := FnCreateNamedPipeW(win.dll_proc(lib, 'CreateNamedPipeW') or { panic(err) })
	connect_pipe := FnConnectNamedPipe(win.dll_proc(lib, 'ConnectNamedPipe') or { panic(err) })
	disconnect := FnDisconnectNamedPipe(win.dll_proc(lib, 'DisconnectNamedPipe') or { panic(err) })
	for {
		h := create(test_pipe.to_wide(), 3, 0, 255, 65536, 65536, 0, unsafe { nil })
		if h == voidptr(-1) {
			return
		}
		connect_pipe(h, unsafe { nil })
		spawn serve_agent_client(h, key, disconnect)
	}
}

fn serve_agent_client(h voidptr, key PrivateKey, disconnect FnDisconnectNamedPipe) {
	mut a := &PipeAgent{
		handle: h
	}
	defer {
		disconnect(h)
		a.close()
	}
	mut head := []u8{len: 4}
	for {
		a.read_full(mut head) or { return }
		n := int(be32_of(head))
		mut req := []u8{len: n}
		a.read_full(mut req) or { return }
		mut reply := new_writer(512)
		if req[0] == agent_request_identities {
			reply.put_u8(agent_identities_answer)
			reply.put_u32(1)
			reply.put_bytes(key.public_blob)
			reply.put_str('quadra-fake-agent')
		} else {
			mut r := new_reader(req[1..])
			r.get_bytes() or { return }
			data := r.get_bytes() or { return }
			sig := key.sign(key.kind, data) or { return }
			reply.put_u8(agent_sign_response)
			reply.put_bytes(sig)
		}
		mut frame := new_writer(reply.buf.len + 4)
		frame.put_bytes(reply.buf)
		sys.pipe_write_all(h, frame.buf) or { return }
	}
}

fn test_agent_auth_and_forwarding() {
	dir, port := gate() or { return }
	key := load_private_key(os.join_path(dir, 'id_ed25519'), none) or { panic(err) }
	spawn fake_agent(key)
	time.sleep(100 * time.millisecond)
	mut cfg := base_config(dir, port, AuthMethod(AgentAuth{}), 'agent')
	cfg.agent_pipe = test_pipe
	cfg.agent_forwarding = true
	mut warned := &EventLog{}
	cfg.warning_ctx = warned
	cfg.on_warning = fn (ctx voidptr, m string) {
		mut warned := unsafe { &EventLog(ctx) }
		warned.warnings << m
	}
	mut s := connect(cfg) or { panic(err) }
	mut got := &ShellSink{}
	mut pty := s.attach(ptyopts(mut got)) or { panic(err) }
	pty.write('ssh-add -L; echo agent-$((40+2))\r'.bytes()) or { panic(err) }
	for _ in 0 .. 500 {
		if got.text().contains('agent-42') {
			break
		}
		time.sleep(20 * time.millisecond)
	}
	text := got.text()
	assert warned.warnings.len == 0, '${warned.warnings}'
	// the remote ssh-add read our identity back through the forwarded channel
	pub_key := os.read_file(os.join_path(dir, 'id_ed25519.pub')) or { panic(err) }.fields()
	assert text.contains(pub_key[1]), text
	pty.close()
}

fn test_agent_unreachable_names_the_pipe() {
	dir, port := gate() or { return }
	mut cfg := base_config(dir, port, AuthMethod(AgentAuth{}), 'noagent')
	connect(cfg) or {
		assert err.msg().contains('no SSH agent is reachable'), err.msg()
		return
	}
	assert false
}
