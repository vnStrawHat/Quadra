// The byte stream under a transport: TCP, or a channel of another connection (specs/23 D4).
module ssh

import net
import sync

fn C.setsockopt(sock int, level int, optname int, optval voidptr, optlen int) int

pub interface Stream {
mut:
	read(mut buf []u8) !int // at least one byte, or an error (io.Eof at end of stream)
	write(data []u8) !
	close()
}

@[heap]
struct TcpStream {
mut:
	conn   &net.TcpConn
	mu     &sync.Mutex = sync.new_mutex()
	closed bool
}

fn new_tcp_stream(mut conn net.TcpConn) &TcpStream {
	conn.set_read_timeout(net.infinite_timeout)
	conn.set_write_timeout(net.infinite_timeout)
	on := int(1)
	C.setsockopt(conn.sock.handle, C.IPPROTO_TCP, C.TCP_NODELAY, voidptr(&on), sizeof(int))
	return &TcpStream{
		conn: conn
	}
}

fn dial_stream(host string, port int) !&TcpStream {
	mut addr := '${host}:${port}'
	if host.contains(':') {
		addr = '[${host}]:${port}'
	}
	mut conn := net.dial_tcp(addr)!
	return new_tcp_stream(mut conn)
}

fn (mut s TcpStream) read(mut buf []u8) !int {
	return s.conn.read(mut buf)
}

fn (mut s TcpStream) write(data []u8) ! {
	s.conn.write(data)!
}

fn (mut s TcpStream) close() {
	s.mu.@lock()
	if s.closed {
		s.mu.unlock()
		return
	}
	s.closed = true
	s.mu.unlock()
	s.conn.close() or {}
}
