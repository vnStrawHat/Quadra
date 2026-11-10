// Reaching the local agent: the named pipe, then Pageant (the transports are in quadra/native/sys).
module ssh

import quadra.native.sys

// connect_agent finds the agent in the same order as OpenSSH: the pipe, then Pageant.
fn connect_agent(pipe string) !AgentLink {
	h := sys.open_pipe_rw(pipe) or {
		pipe_err := err.code()
		pageant := sys.open_pageant() or {
			return auth_err('no SSH agent is reachable (${pipe}: error ${pipe_err}; Pageant: ${err})')
		}
		return AgentLink(&PageantAgent{
			pageant: pageant
		})
	}
	return AgentLink(&PipeAgent{
		handle: h
	})
}

@[heap]
struct PipeAgent {
mut:
	handle voidptr
}

fn (mut a PipeAgent) read_full(mut buf []u8) ! {
	sys.pipe_read_full(a.handle, mut buf)!
}

fn (mut a PipeAgent) query(request []u8) ![]u8 {
	mut w := new_writer(request.len + 4)
	w.put_bytes(request)
	sys.pipe_write_all(a.handle, w.buf)!
	mut head := []u8{len: 4}
	a.read_full(mut head)!
	n := int(u32(head[0]) << 24 | u32(head[1]) << 16 | u32(head[2]) << 8 | u32(head[3]))
	if n < 1 || n > agent_max_message {
		return error('bad agent reply length')
	}
	mut body := []u8{len: n}
	a.read_full(mut body)!
	return body
}

fn (mut a PipeAgent) close() {
	if a.handle != unsafe { nil } {
		sys.close_handle(a.handle)
		a.handle = unsafe { nil }
	}
}

@[heap]
struct PageantAgent {
mut:
	pageant &sys.Pageant
}

fn (mut a PageantAgent) query(request []u8) ![]u8 {
	mut w := new_writer(request.len + 4)
	w.put_bytes(request)
	return a.pageant.query(w.buf)
}

fn (mut a PageantAgent) close() {}
