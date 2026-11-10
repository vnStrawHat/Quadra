// Inbound messages of a connection: replies, channel traffic, server-opened channels.
module ssh

fn (mut c Conn) dispatch(pkt []u8) ! {
	mut r := new_reader(pkt[1..])
	match pkt[0] {
		msg_request_success, msg_request_failure {
			c.mu.@lock()
			if c.globals.len == 0 {
				c.mu.unlock()
				return
			}
			g := c.globals.first()
			c.globals.delete(0)
			c.mu.unlock()
			if g.waiter != unsafe { nil } {
				mut w := g.waiter
				w.finish(pkt[0] == msg_request_success, pkt[1..])
			}
		}
		msg_global_request {
			r.get_str()! // not supported: answer failure if asked
			if r.get_bool()! {
				c.tr.send([msg_request_failure])!
			}
		}
		msg_channel_open {
			c.channel_open_request(mut r)!
		}
		else {
			c.dispatch_channel(pkt[0], mut r)!
		}
	}
}

fn (mut c Conn) dispatch_channel(kind u8, mut r Reader) ! {
	id := r.get_u32()!
	mut ch := c.channel(id) or { return }
	match kind {
		msg_channel_open_confirm {
			remote := r.get_u32()!
			window := r.get_u32()!
			packet := r.get_u32()!
			ch.confirmed(remote, window, packet)
		}
		msg_channel_open_failure {
			r.get_u32()!
			desc := r.get_str() or { '' }
			ch.open_wait.finish(false, desc.bytes())
		}
		msg_window_adjust {
			ch.window_add(r.get_u32()!)
		}
		msg_channel_data {
			ch.receive(r.get_bytes()!)
		}
		msg_channel_extended {
			r.get_u32()!
			ch.receive(r.get_bytes()!)
		}
		msg_channel_eof {
			ch.got_eof()
		}
		msg_channel_close {
			ch.got_close()!
			c.remove_channel(id)
		}
		msg_channel_success, msg_channel_failure {
			ch.request_reply(kind == msg_channel_success)
		}
		msg_channel_request {
			name := r.get_str()!
			want := r.get_bool()!
			ch.incoming_request(name, mut r)
			if want {
				c.channel_reply(ch.remote_id, false)!
			}
		}
		else {
			c.tr.send([msg_unimplemented])!
		}
	}
}

fn (mut c Conn) channel_reply(remote u32, ok bool) ! {
	mut w := new_writer(8)
	w.put_u8(if ok { msg_channel_success } else { msg_channel_failure })
	w.put_u32(remote)
	c.tr.send(w.buf)!
}

// channel_open_request handles a channel the server opens: refused unless asked for.
fn (mut c Conn) channel_open_request(mut r Reader) ! {
	kind := r.get_str()!
	remote := r.get_u32()!
	window := r.get_u32()!
	packet := r.get_u32()!
	if kind == 'forwarded-tcpip' && c.forwards != unsafe { nil } {
		addr := r.get_str()!
		port := r.get_u32()!
		mut f := c.forwards
		if target := f.remote_target(addr, port) {
			mut ch := c.accept_channel(remote, window, packet)
			c.send_confirm(ch)! // before any data of the relay
			spawn f.relay_outbound(mut ch, target)
			return
		}
	} else if kind == 'auth-agent@openssh.com' && c.agent_on {
		mut ch := c.accept_channel(remote, window, packet)
		c.send_confirm(ch)!
		spawn bridge_agent(mut ch, c.agent_pipe)
		return
	}
	mut w := new_writer(32)
	w.put_u8(msg_channel_open_failure)
	w.put_u32(remote)
	w.put_u32(1) // administratively prohibited
	w.put_str('')
	w.put_str('')
	c.tr.send(w.buf)!
}

fn (mut c Conn) accept_channel(remote u32, window u32, packet u32) &Channel {
	c.mu.@lock()
	id := c.next_id
	c.next_id++
	mut ch := new_channel(c, id, default_window)
	ch.remote_id = remote
	ch.remote_window = u64(window)
	ch.max_packet = packet
	c.channels[id] = ch
	c.mu.unlock()
	return ch
}

fn (mut c Conn) send_confirm(ch &Channel) ! {
	mut w := new_writer(20)
	w.put_u8(msg_channel_open_confirm)
	w.put_u32(ch.remote_id)
	w.put_u32(ch.id)
	w.put_u32(ch.window_initial)
	w.put_u32(default_max_packet)
	c.tr.send(w.buf)!
}
