// A channel as a Stream: the transport of the next hop rides on it (specs/23 D4).
module ssh

@[heap]
struct ChanStream {
mut:
	ch &Channel
}

fn (mut s ChanStream) read(mut buf []u8) !int {
	return s.ch.read(mut buf)
}

fn (mut s ChanStream) write(data []u8) ! {
	s.ch.write(data)!
}

fn (mut s ChanStream) close() {
	s.ch.close()
}

// open_direct_tcpip opens a direct-tcpip channel to host:port on this connection.
fn (mut c Conn) open_direct_tcpip(host string, port int, from string, from_port int) !&Channel {
	mut w := new_writer(host.len + from.len + 16)
	w.put_str(host)
	w.put_u32(u32(port))
	w.put_str(from)
	w.put_u32(u32(from_port))
	return c.open_channel('direct-tcpip', w.buf, default_window)
}
