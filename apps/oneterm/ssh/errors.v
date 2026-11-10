// Errors the UI can match, and the cancellation token (specs/23).
module ssh

import sync

// ConnectPhase names the step of a connection attempt that failed.
pub enum ConnectPhase {
	transport
	authentication
	channel_open
	pty_request
	shell_request
	shell_integration
	sftp_setup
}

pub fn (p ConnectPhase) label() string {
	return match p {
		.transport { 'connect' }
		.authentication { 'authentication' }
		.channel_open { 'channel open' }
		.pty_request { 'PTY request' }
		.shell_request { 'shell request' }
		.shell_integration { 'shell integration' }
		.sftp_setup { 'SFTP setup' }
	}
}

pub struct ConnectError {
	Error
pub:
	phase   ConnectPhase
	message string
}

pub fn (e ConnectError) msg() string {
	return 'SSH ${e.phase.label()} failed: ${e.message}'
}

// HostKeyUnknown means the key is not in known_hosts: the UI may ask, then retry with
// accept_new(fingerprint).
pub struct HostKeyUnknown {
	Error
pub:
	host        string
	port        int
	algorithm   string
	fingerprint string
}

pub fn (e HostKeyUnknown) msg() string {
	return 'Unknown SSH host key for ${e.host}:${e.port} (${e.algorithm}), SHA-256 fingerprint: ${e.fingerprint}'
}

// HostKeyChanged is never approvable: the user removes the stale entry by hand.
pub struct HostKeyChanged {
	Error
pub:
	host        string
	port        int
	fingerprint string
}

pub fn (e HostKeyChanged) msg() string {
	return 'SSH host key changed for ${e.host}:${e.port}; refusing connection (SHA-256 fingerprint: ${e.fingerprint})'
}

pub struct Cancelled {
	Error
}

pub fn (e Cancelled) msg() string {
	return 'cancelled'
}

// SftpStatus is the SSH_FX_* code of a failed request.
pub enum SftpStatus {
	other
	eof
	no_such_file
	permission_denied
	failure
	bad_message
	connection_lost
	op_unsupported
}

pub struct SftpError {
	Error
pub:
	status  SftpStatus
	message string
}

pub fn (e SftpError) msg() string {
	if e.message == '' {
		return 'SFTP ${e.status}'
	}
	return 'SFTP ${e.status}: ${e.message}'
}

// phase_err wraps a failure as a ConnectError, keeping typed errors as they are.
fn phase_err(phase ConnectPhase, err IError) IError {
	if err is HostKeyUnknown || err is HostKeyChanged || err is Cancelled || err is ConnectError {
		return err
	}
	return ConnectError{
		phase:   phase
		message: err.msg()
	}
}

// Cancel stops a connection attempt or a transfer from any thread.
@[heap]
pub struct Cancel {
mut:
	mu   &sync.Mutex = sync.new_mutex()
	flag bool
	wake &sync.Semaphore = unsafe { nil } // posted once to wake a blocked operation
}

pub fn new_cancel() &Cancel {
	return &Cancel{}
}

pub fn (mut c Cancel) cancel() {
	c.mu.@lock()
	already := c.flag
	c.flag = true
	mut wake := c.wake
	c.mu.unlock()
	if !already && wake != unsafe { nil } {
		wake.post()
	}
}

pub fn (mut c Cancel) is_cancelled() bool {
	c.mu.@lock()
	defer {
		c.mu.unlock()
	}
	return c.flag
}

// set_wake posts wake on cancel, at once if already cancelled (a semaphore, not a closure:
// no closures in apps, specs/14).
fn (mut c Cancel) set_wake(wake &sync.Semaphore) {
	c.mu.@lock()
	c.wake = wake
	done := c.flag
	c.mu.unlock()
	if done {
		mut w := unsafe { wake }
		w.post()
	}
}
