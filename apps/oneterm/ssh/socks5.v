// SOCKS5 (RFC 1928): no authentication, CONNECT only, destinations resolved on the far side.
module ssh

import net
import time

fn C.shutdown(sock int, how int) int

const socks5_deadline = 5 * time.second

fn tcp_shutdown_send(conn &net.TcpConn) {
	C.shutdown(conn.sock.handle, 1)
}

fn read_tcp_exact(mut tcp net.TcpConn, n int) ![]u8 {
	mut out := []u8{len: n}
	mut got := 0
	for got < n {
		k := tcp.read(mut out[got..])!
		if k == 0 {
			return error('unexpected end of stream')
		}
		got += k
	}
	return out
}

// socks5_reply is a reply with an all-zero IPv4 bind address.
fn socks5_reply(rep u8) []u8 {
	return [u8(5), rep, 0, 1, 0, 0, 0, 0, 0, 0]
}

fn hex_group(a u8, b u8) string {
	return '${(u16(a) << 8 | u16(b)):x}'
}

// socks5_target runs the greeting and the CONNECT request and returns the destination.
// The success reply is sent by the caller once the channel is open.
fn socks5_target(mut tcp net.TcpConn) !(string, int) {
	tcp.set_read_timeout(socks5_deadline)
	defer {
		tcp.set_read_timeout(net.infinite_timeout)
	}
	head := read_tcp_exact(mut tcp, 2)!
	if head[0] != 5 {
		return error('not SOCKS5')
	}
	methods := read_tcp_exact(mut tcp, int(head[1]))!
	if 0 !in methods {
		tcp.write([u8(5), 0xff])!
		return error('client offers no no-authentication method')
	}
	tcp.write([u8(5), 0])!
	request := read_tcp_exact(mut tcp, 4)!
	if request[0] != 5 {
		return error('not SOCKS5')
	}
	// Read the whole request before judging it: closing with unread bytes resets the socket
	// on Windows and the reply is lost.
	mut host := ''
	match request[3] {
		1 {
			ip := read_tcp_exact(mut tcp, 4)!
			host = '${ip[0]}.${ip[1]}.${ip[2]}.${ip[3]}'
		}
		3 {
			n := read_tcp_exact(mut tcp, 1)!
			host = read_tcp_exact(mut tcp, int(n[0]))!.bytestr()
		}
		4 {
			ip := read_tcp_exact(mut tcp, 16)!
			mut groups := []string{}
			for i := 0; i < 16; i += 2 {
				groups << hex_group(ip[i], ip[i + 1])
			}
			host = groups.join(':')
		}
		else {
			tcp.write(socks5_reply(8))!
			return error('unknown address type')
		}
	}

	port_bytes := read_tcp_exact(mut tcp, 2)!
	port := int(u16(port_bytes[0]) << 8 | u16(port_bytes[1]))
	if request[1] != 1 {
		tcp.write(socks5_reply(7))!
		return error('only CONNECT is supported')
	}
	return host, port
}
