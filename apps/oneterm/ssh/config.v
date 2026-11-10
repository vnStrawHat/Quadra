// What a connection needs (specs/23). Everything is plain data built by the UI.
module ssh

pub const max_jump_hops = 4

pub const default_agent_pipe = r'\\.\pipe\openssh-ssh-agent'

// Secret keeps a password or passphrase out of logs and can be wiped.
pub struct Secret {
mut:
	bytes []u8
}

pub fn secret(s string) Secret {
	return Secret{
		bytes: s.bytes()
	}
}

pub fn (s Secret) str() string {
	return '***'
}

pub fn (s Secret) is_empty() bool {
	return s.bytes.len == 0
}

pub fn (mut s Secret) wipe() {
	wipe(mut s.bytes)
}

fn wipe(mut b []u8) {
	for i in 0 .. b.len {
		b[i] = 0
	}
}

pub struct NoAuth {}

pub struct PasswordAuth {
pub mut:
	password Secret
}

pub struct KeyAuth {
pub mut:
	key_path   string
	passphrase ?Secret
}

pub struct AgentAuth {}

pub type AuthMethod = AgentAuth | KeyAuth | NoAuth | PasswordAuth

// HostKeyPolicy for a key not yet in known_hosts: strict (fingerprint empty) fails closed;
// otherwise exactly this SHA-256 fingerprint is trusted and recorded.
pub struct HostKeyPolicy {
pub:
	fingerprint string
}

pub fn accept_new(fingerprint string) HostKeyPolicy {
	return HostKeyPolicy{
		fingerprint: fingerprint
	}
}

pub struct Keepalive {
pub:
	enabled bool = true
	seconds int  = 30
	max     int  = 3
}

fn clamp_int(v int, lo int, hi int) int {
	if v < lo {
		return lo
	}
	if v > hi {
		return hi
	}
	return v
}

fn (k Keepalive) normalized() Keepalive {
	return Keepalive{
		enabled: k.enabled
		seconds: clamp_int(k.seconds, 5, 3600)
		max:     clamp_int(k.max, 1, 20)
	}
}

// Hop is one host on the route: a jump host, or the target itself.
pub struct Hop {
pub mut:
	host     string
	port     int = 22
	username string
	auth     AuthMethod = AuthMethod(NoAuth{})
	policy   HostKeyPolicy
}

pub fn (h Hop) label() string {
	return '${h.username}@${h.host}:${h.port}'
}

// LocalForward listens on bind:bind_port and connects each client to the target on the far side.
pub struct LocalForward {
pub:
	bind        string = '127.0.0.1'
	bind_port   int
	target_host string
	target_port int
}

// RemoteForward asks the server to listen and connects each client to a local target.
pub struct RemoteForward {
pub:
	bind_host   string = '127.0.0.1'
	bind_port   int // 0: the server picks
	target_host string
	target_port int
}

// DynamicForward is a local SOCKS5 proxy resolving destinations on the far side.
pub struct DynamicForward {
pub:
	bind      string = '127.0.0.1'
	bind_port int
}

pub type PortForward = DynamicForward | LocalForward | RemoteForward

// Config is one connection: the target hop, the hops before it and what to run.
@[heap]
pub struct Config {
pub mut:
	target            Hop
	jump_hops         []Hop
	known_hosts_path  string // required: the OpenSSH known_hosts file
	agent_pipe        string = default_agent_pipe
	agent_forwarding  bool
	shell_integration bool
	port_forwards     []PortForward
	keepalive         Keepalive
	cols              int             = 80
	rows              int             = 24
	term_version      string          = '0.6.11'
	cancel            &Cancel         = new_cancel()
	on_warning        fn (msg string) = unsafe { nil } // a forward or agent problem: a toast
}
