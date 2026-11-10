// connect: the route, the shell, SFTP and forwards; the teardown of a session (specs/23).
module ssh

import sync
import quadra.platform

const term_modes_echo_off = [u8(53), 0, 0, 0, 0, 0] // ECHO = 0, then TTY_OP_END
const term_modes_default = [u8(0)]

// Session is one connected SSH target with its shell, SFTP channel and forwards.
@[heap]
pub struct Session {
mut:
	mu        &sync.Mutex = sync.new_mutex()
	conn      &Conn
	hops      []&Conn
	shell     &ShellPty = unsafe { nil }
	sftp_     &Sftp     = unsafe { nil }
	forwards  &Forwards = unsafe { nil }
	traffic   &Traffic  = &Traffic{}
	attached  bool
	closed    bool
	on_closed fn () = unsafe { nil }
}

fn warn(cfg &Config, msg string) {
	if cfg.on_warning != unsafe { nil } {
		cfg.on_warning(cfg.warning_ctx, msg)
	}
}

// connect establishes everything; call it on a worker thread (specs/23 D6).
pub fn connect(cfg Config) !&Session {
	mut config := unsafe { &cfg }
	if config.jump_hops.len > max_jump_hops {
		return error('too many jump hosts')
	}
	mut g := new_guard(config.cancel)
	spawn g.watch()
	defer {
		g.finish()
	}
	mut hops := []&Conn{}
	mut last := unsafe { &Conn(nil) }
	for i in 0 .. config.jump_hops.len {
		mut hop := config.jump_hops[i]
		label := hop.label()
		mut conn := connect_hop(mut g, last, mut hop, config) or {
			close_all(mut hops)
			return hop_error(label, IError(err))
		}
		hops << conn
		last = conn
	}
	mut session := build_session(mut g, last, mut hops, config) or {
		close_all(mut hops)
		return err
	}
	return session
}

fn close_all(mut hops []&Conn) {
	for i := hops.len - 1; i >= 0; i-- {
		mut c := hops[i]
		c.close()
	}
}

fn build_session(mut g Guard, last &Conn, mut hops []&Conn, cfg &Config) !&Session {
	mut target := cfg.target
	mut conn := connect_hop(mut g, last, mut target, cfg)!
	mut s := &Session{
		conn: conn
		hops: hops
	}
	s.conn.owner = s
	s.open_shell(mut g, cfg) or {
		s.close()
		return err
	}
	s.open_sftp_channel(mut g, cfg) or {
		// a cancelled attempt fails as a whole; a missing SFTP server leaves a terminal
		if err is Cancelled {
			s.close()
			return err
		}
	}
	s.forwards = new_forwards(s.conn, cfg)
	if cfg.agent_forwarding {
		s.conn.agent_pipe = cfg.agent_pipe
		s.conn.agent_on = true
	}
	return s
}

fn (mut s Session) open_shell(mut g Guard, cfg &Config) ! {
	g.enter(.channel_open)
	mut ch := s.conn.open_channel('session', [], default_window) or { return g.fail(err) }
	ch.traffic = s.traffic
	if cfg.agent_forwarding {
		ok := ch.request('auth-agent-req@openssh.com', [], true) or { return g.fail(err) }
		if !ok {
			warn(cfg, 'The server refused agent forwarding for ${cfg.target.label()}.')
		}
	}
	g.enter(.pty_request)
	mut pty := new_writer(64)
	pty.put_str('xterm-256color')
	pty.put_u32(u32(cfg.cols))
	pty.put_u32(u32(cfg.rows))
	pty.put_u32(0)
	pty.put_u32(0)
	if cfg.shell_integration {
		pty.put_bytes(term_modes_echo_off)
	} else {
		pty.put_bytes(term_modes_default)
	}
	ok := ch.request('pty-req', pty.buf, true) or { return g.fail(err) }
	if !ok {
		return ConnectError{
			phase:   .pty_request
			message: 'the server refused a PTY'
		}
	}
	for pair in [['COLORTERM', 'truecolor'], ['TERM_PROGRAM', 'OneTerm'],
		['TERM_PROGRAM_VERSION', cfg.term_version]] {
		mut env := new_writer(64)
		env.put_str(pair[0])
		env.put_str(pair[1])
		ch.request('env', env.buf, false) or { return g.fail(err) }
	}
	g.enter(.shell_request)
	shell_ok := ch.request('shell', [], true) or { return g.fail(err) }
	if !shell_ok {
		return ConnectError{
			phase:   .shell_request
			message: 'the server refused a shell'
		}
	}
	if cfg.shell_integration {
		g.enter(.shell_integration)
		line := '${shell_integration_bootstrap}\r'
		ch.write(line.bytes()) or { return g.fail(err) }
	}
	mut pty_ := new_shell_pty(ch)
	pty_.session = s
	s.shell = pty_
}

fn (mut s Session) open_sftp_channel(mut g Guard, _ &Config) ! {
	g.enter(.sftp_setup)
	mut ch := s.conn.open_channel('session', [], sftp_window) or { return g.fail(err) }
	ch.traffic = s.traffic
	mut sub := new_writer(16)
	sub.put_str('sftp')
	ok := ch.request('subsystem', sub.buf, true) or { return g.fail(err) }
	if !ok {
		ch.close()
		return error('the server has no sftp subsystem')
	}
	s.sftp_ = new_sftp(mut ch) or {
		ch.close()
		return g.fail(err)
	}
}

// attach starts delivering shell output: use it as term.start's spawn function.
pub fn (mut s Session) attach(opts platform.PtyOptions) !platform.Pty {
	s.mu.@lock()
	defer {
		s.mu.unlock()
	}
	if s.attached {
		return error('this session is already attached')
	}
	s.attached = true
	mut p := s.shell
	p.start(opts)
	return platform.Pty(p)
}

// sftp returns the SFTP client, or none when the server has no SFTP subsystem.
pub fn (s &Session) sftp() ?&Sftp {
	if s.sftp_ == unsafe { nil } {
		return none
	}
	return s.sftp_
}

// bytes_in_out returns the shell and SFTP payload bytes received and sent.
pub fn (s &Session) bytes_in_out() (u64, u64) {
	mut t := s.traffic
	return t.snapshot()
}

// close tears everything down: forwards, SFTP, the target, then the hops innermost-out.
pub fn (mut s Session) close() {
	s.mu.@lock()
	if s.closed {
		s.mu.unlock()
		return
	}
	s.closed = true
	s.mu.unlock()
	if s.forwards != unsafe { nil } {
		s.forwards.stop()
	}
	if s.sftp_ != unsafe { nil } {
		s.sftp_.close()
	}
	s.conn.close()
	close_all(mut s.hops)
}
