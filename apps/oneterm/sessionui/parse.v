// The one parser for `user[@host[:port]]` strings and port fields (specs/26).
module sessionui

// UserHostPort is the parts of a `user[@host[:port]]` string; missing parts are none.
pub struct UserHostPort {
pub:
	user string
	host ?string
	port ?int
}

// parse_port accepts 1..65535, surrounding whitespace ignored.
pub fn parse_port(text string) !int {
	t := text.trim_space()
	if t != '' && t.len <= 5 && t.contains_only('0123456789') {
		n := t.int()
		if n >= 1 && n <= 65535 {
			return n
		}
	}
	return error('Port "${t}" is not a number between 1 and 65535.')
}

fn split_host_port(s string) !(string, ?int) {
	if s.starts_with('[') {
		end := s.index(']') or {
			return error('IPv6 host must be enclosed in brackets, e.g. [::1]:22.')
		}
		host := s[1..end]
		if host == '' {
			return error("Host is required after the '@'.")
		}
		after := s[end + 1..]
		if after == '' {
			return host, ?int(none)
		}
		if !after.starts_with(':') {
			return error('Port "${after}" is not a number between 1 and 65535.')
		}
		return host, ?int(parse_port(after[1..])!)
	}
	if s.count(':') >= 2 {
		return s, ?int(none) // a bare IPv6 address, no port
	}
	mut host := s
	mut port := ?int(none)
	if i := s.last_index(':') {
		host = s[..i]
		port = parse_port(s[i + 1..])!
	}
	if host == '' {
		return error("Host is required after the '@'.")
	}
	return host, port
}

// parse_user_host_port reads `user`, `user@host` or `user@host:port`. An invalid port is an
// error, never a silent default and never folded into the host name.
pub fn parse_user_host_port(raw string) !UserHostPort {
	text := raw.trim_space()
	at := text.index('@') or { return UserHostPort{
		user: text
	} }
	if at == 0 {
		return error("Username is required before the '@'.")
	}
	host, port := split_host_port(text[at + 1..])!
	return UserHostPort{
		user: text[..at]
		host: host
		port: port
	}
}

// ConnectTarget is the result of the Quick Connect fields.
pub struct ConnectTarget {
pub:
	username string
	host     string
	port     int
}

pub fn (t ConnectTarget) label() string {
	return '${t.username}@${t.host}:${t.port}'
}

// resolve_target combines Username (`user[@host[:port]]`), Host and Port. An explicit field
// wins over the parsed part; an empty Port falls back to the parsed port, then to 22.
pub fn resolve_target(username_raw string, host_field string, port_field string) !ConnectTarget {
	parsed := parse_user_host_port(username_raw)!
	mut host := host_field
	if host == '' {
		host = parsed.host or { return error('Host is required.') }
	}
	mut port := 22
	if port_field != '' {
		port = parse_port(port_field)!
	} else if p := parsed.port {
		port = p
	}
	if parsed.user == '' {
		return error('Username is required.')
	}
	return ConnectTarget{parsed.user, host, port}
}
