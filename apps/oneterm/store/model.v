// The saved SSH sessions: ssh_session.json's record types (specs/25).
module store

pub const default_port = 22
pub const default_color_hex = '#56B6C2'
pub const max_jump_hops = 4

pub enum AuthMethod {
	password    // ask for an optional password when connecting
	private_key // a key file, and an optional passphrase asked when connecting
	agent       // an identity held by the local SSH agent
}

// LoggingOverride is a saved session's override of automatic terminal logging.
pub enum LoggingOverride {
	inherit
	on
	off
}

// resolve applies the override to the global SSH logging setting.
pub fn (o LoggingOverride) resolve(global bool) bool {
	return match o {
		.inherit { global }
		.on { true }
		.off { false }
	}
}

pub enum ForwardKind {
	local
	remote
	dynamic
}

// PortForward: a local listener relayed from the remote side, a remote listener relayed
// locally, or a local SOCKS5 proxy. `bind` is an IP for local and dynamic, a host for remote.
pub struct PortForward {
pub mut:
	kind        ForwardKind
	bind        string = '127.0.0.1'
	bind_port   int
	target_host string
	target_port int
}

pub enum ForwardError {
	bind_port
	target_port
	empty_host
	whitespace
}

fn host_problem(host string) ?ForwardError {
	if host == '' {
		return .empty_host
	}
	if host.contains_any(' \t\r\n') {
		return .whitespace
	}
	return none
}

// validate returns what is unusable in the forward, none when it is fine.
pub fn (f PortForward) validate() ?ForwardError {
	match f.kind {
		.local {
			if f.bind_port == 0 {
				return .bind_port
			}
			if f.target_port == 0 {
				return .target_port
			}
			return host_problem(f.target_host)
		}
		.remote {
			if f.target_port == 0 {
				return .target_port
			}
			if p := host_problem(f.bind) {
				return p
			}
			return host_problem(f.target_host)
		}
		.dynamic {
			if f.bind_port == 0 {
				return .bind_port
			}
			return none
		}
	}
}

// kind_label is the dialog's label.
pub fn (f PortForward) kind_label() string {
	return match f.kind {
		.local { 'Local' }
		.remote { 'Remote' }
		.dynamic { 'Dynamic' }
	}
}

// bind_key identifies the listening side, to reject duplicates.
pub fn (f PortForward) bind_key() string {
	return '${f.kind_label()}|${f.bind}|${f.bind_port}'
}

// binds_loopback reports a listener confined to this machine.
pub fn (f PortForward) binds_loopback() bool {
	if f.kind == .remote {
		return f.bind in ['127.0.0.1', '::1', 'localhost']
	}
	return f.bind == '::1' || f.bind.starts_with('127.')
}

// summary is one line such as `L 127.0.0.1:8080 -> localhost:80`.
pub fn (f PortForward) summary() string {
	return match f.kind {
		.local { 'L ${f.bind}:${f.bind_port} -> ${f.target_host}:${f.target_port}' }
		.remote { 'R ${f.bind}:${f.bind_port} -> ${f.target_host}:${f.target_port}' }
		.dynamic { 'D ${f.bind}:${f.bind_port} (SOCKS5)' }
	}
}

// Session is the user-editable record of one saved SSH connection.
pub struct Session {
pub mut:
	label            string
	host             string
	port             int = default_port
	username         ?string
	auth_method      AuthMethod
	key_path         ?string // never a passphrase
	color            ?string // hex string such as #58c4dc
	group            ?string
	logging          LoggingOverride
	jump_host        ?u64 // id of the saved session to connect through first
	forwards         []PortForward
	agent_forwarding bool
	// The login password sealed under the vault, opaque here and written back byte for byte.
	sealed_password ?string
}

// has_address reports whether a password sealed for host, port, username belongs here: a
// password is never kept for an unknown user, a changed address or another auth method.
pub fn (s Session) has_address(host string, port int, username string) bool {
	return s.auth_method == .password && (s.username or { '' }) == username && username != ''
		&& s.host == host && s.port == port
}

// saved_password is the sealed value for a session that authenticates with a password.
pub fn (s Session) saved_password() ?string {
	if s.auth_method != .password {
		return none
	}
	return s.sealed_password
}

// Entry is one row: a stable id, never reused within one document, plus the record.
pub struct Entry {
pub mut:
	id      u64
	session Session
}

// JumpError says why a jump_host chain cannot be followed.
pub struct JumpError {
	Error
pub:
	kind  JumpErrorKind
	id    u64
	count int
}

pub enum JumpErrorKind {
	missing
	cycle
	too_long
}

pub fn (e JumpError) msg() string {
	return match e.kind {
		.missing { 'jump host session ${e.id} no longer exists' }
		.cycle { 'the jump host chain loops back on itself' }
		.too_long { 'the jump host chain has ${e.count} hops; at most ${max_jump_hops} are supported' }
	}
}
