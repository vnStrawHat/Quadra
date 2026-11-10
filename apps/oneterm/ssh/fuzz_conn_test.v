// Fuzz: connection-protocol messages from the server (channel traffic, global requests, opens).
module ssh

import quadra.testing.fuzz

fn fuzz_conn() &Conn {
	mut c := new_conn(fuzz_transport([]))
	for id in 0 .. 3 {
		mut ch := new_channel(c, u32(id), 4096)
		c.channels[u32(id)] = ch
	}
	c.next_id = 3
	return c
}

fn chan_msg(kind u8, id u32, rest []u8) []u8 {
	mut w := new_writer(rest.len + 8)
	w.put_u8(kind)
	w.put_u32(id)
	w.put_raw(rest)
	return w.buf
}

fn data_msg(id u32, n int) []u8 {
	mut w := new_writer(n + 16)
	w.put_u8(msg_channel_data)
	w.put_u32(id)
	w.put_bytes([]u8{len: n, init: `x`})
	return w.buf
}

fn test_data_beyond_the_window_is_an_error() {
	// a server that ignores the window grew the receive buffer without bound
	mut c := fuzz_conn()
	c.dispatch(data_msg(0, 4096)) or { panic(err) }
	if _ := c.dispatch(data_msg(0, 1)) {
		assert false, 'data past the window was accepted'
	}
	// reading grants the window back
	mut ch := c.channel(1) or { panic(err) }
	c.dispatch(data_msg(1, 4096)) or { panic(err) }
	mut buf := []u8{len: 4096}
	ch.read_full(mut buf) or { panic(err) }
	c.dispatch(data_msg(1, 4096)) or { panic(err) }
}

fn test_fuzz_connection_messages() {
	n := fuzz.iters(500)
	mut rng := fuzz.new_rng(0xc4a)
	mut req := new_writer(64)
	req.put_str('exit-status')
	req.put_bool(true)
	req.put_u32(3)
	mut sig := new_writer(64)
	sig.put_str('exit-signal')
	sig.put_bool(false)
	sig.put_str('KILL')
	mut open := new_writer(64)
	open.put_u8(msg_channel_open)
	open.put_str('forwarded-tcpip')
	open.put_u32(7)
	open.put_u32(4096)
	open.put_u32(32768)
	open.put_str('127.0.0.1')
	open.put_u32(80)
	mut glob := new_writer(64)
	glob.put_u8(msg_global_request)
	glob.put_str('hostkeys-00@openssh.com')
	glob.put_bool(true)
	mut win := new_writer(16)
	win.put_u32(0xffffffff)
	corpus := [
		chan_msg(msg_channel_open_confirm, 0, [u8(0), 0, 0, 9, 0, 0, 16, 0, 0, 0, 128, 0]),
		chan_msg(msg_channel_open_failure, 1, [u8(0), 0, 0, 1, 0, 0, 0, 2, `n`, `o`]),
		chan_msg(msg_window_adjust, 2, win.buf),
		data_msg(0, 100),
		chan_msg(msg_channel_extended, 1, [u8(0), 0, 0, 1, 0, 0, 0, 2, `e`, `r`]),
		chan_msg(msg_channel_eof, 0, []),
		chan_msg(msg_channel_close, 2, []),
		chan_msg(msg_channel_success, 1, []),
		chan_msg(msg_channel_failure, 0, []),
		chan_msg(msg_channel_request, 0, req.buf),
		chan_msg(msg_channel_request, 1, sig.buf),
		open.buf,
		glob.buf,
		[msg_request_success, 0, 0, 0, 1],
		[msg_request_failure],
		[u8(126)],
	]
	mut w := fuzz.watch('ssh connection messages', n)
	for i in 0 .. n {
		mut c := fuzz_conn()
		w.start()
		mut size := 0
		for _ in 0 .. 1 + rng.intn(8) {
			mut p := rng.mutate(corpus[rng.intn(corpus.len)], corpus)
			if p.len == 0 {
				p = [u8(126)]
			}
			size += p.len
			c.dispatch(p) or { break }
		}
		for _, ch in c.channels {
			mut queued := 0
			for chunk in ch.rx {
				queued += chunk.len
			}
			assert queued <= 4096
		}
		w.done(i, size, 2000)
		fuzz_ceiling(i)
	}
	w.finish(64)
}
