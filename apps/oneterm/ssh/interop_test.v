// Live tests against sshd on 127.0.0.1 (specs/23). Run by interop.sh, skipped otherwise.
module ssh

import os
import time

fn test_key_auth_and_exec_all_key_types() {
	dir, port := gate() or { return }
	for name, pass in {
		'id_ed25519':        ''
		'id_ed25519_pass':   'open sesame'
		'id_ed25519_chacha': 'open sesame'
		'id_ed25519_gcm':    'open sesame'
		'id_rsa':            ''
		'id_rsa_pass':       'open sesame'
		'id_rsa_pem':        ''
		'id_rsa_pkcs8':      ''
		'id_rsa_pkcs8_enc':  'open sesame'
		'id_rsa_dek':        'open sesame'
		'id_ecdsa256':       ''
		'id_ecdsa384':       ''
		'id_ecdsa521':       ''
		'id_ecdsa_pem':      ''
	} {
		if !os.exists(os.join_path(dir, name)) {
			continue
		}
		mut s := connect(base_config(dir, port, key_auth(dir, name, pass), 'keys')) or {
			assert false, '${name}: ${err}'
			return
		}
		out, code := run_cmd(mut s.conn, 'echo hello-${name}') or { panic(err) }
		assert out.contains('hello-${name}'), name
		assert code == 0, name
		s.close()
	}
}

fn test_host_key_verdicts() {
	dir, port := gate() or { return }
	auth := key_auth(dir, 'id_ed25519', '')
	mut cfg := base_config(dir, port, auth, 'verdict')
	os.rm(cfg.known_hosts_path) or {}
	connect(cfg) or {
		assert err is HostKeyUnknown, err.msg()
		e := err as HostKeyUnknown
		want := os.read_file(os.join_path(dir, 'hk_ed25519.fp')) or { '' }
		assert e.fingerprint == want.trim_space()
		cfg.target.policy = accept_new(e.fingerprint)
		mut s := connect(cfg) or { panic(err) }
		s.close()
		// recorded: strict now passes
		cfg.target.policy = HostKeyPolicy{}
		mut again := connect(cfg) or { panic(err) }
		again.close()
		return
	}
	assert false, 'an unknown host key must fail closed'
}

fn test_host_key_changed_is_refused() {
	dir, port := gate() or { return }
	mut cfg := base_config(dir, port, key_auth(dir, 'id_ed25519', ''), 'changed')
	other := os.read_file(os.join_path(dir, 'id_ed25519.pub')) or { panic(err) }.fields()
	// the known_hosts holds a different ed25519 key for this host
	os.write_file(cfg.known_hosts_path, '[127.0.0.1]:${port} ${other[0]} ${other[1]}\n') or {
		panic(err)
	}
	connect(cfg) or {
		assert err is HostKeyChanged, err.msg()
		return
	}
	assert false, 'a changed host key must be refused'
}

fn test_wrong_passphrase_and_auth_failures() {
	dir, port := gate() or { return }
	cfg := base_config(dir, port, key_auth(dir, 'id_ed25519_pass', 'nope'), 'badpass')
	connect(cfg) or {
		assert err is ConnectError, err.msg()
		assert err.msg().contains('wrong passphrase') || err.msg().contains('passphrase')
		// password path: sshd cannot verify it without the account password, so this exercises
		// USERAUTH_FAILURE and the remaining-method message.
		mut pw := base_config(dir, port, PasswordAuth{
			password: secret('not the password')
		}, 'pw')
		connect(pw) or {
			assert err.msg().contains('the server accepts'), err.msg()
			return
		}
		assert false, 'a wrong password must fail'
		return
	}
	assert false
}

// connect_offering connects with a restricted algorithm offer; returns the output of a command.
fn connect_offering(dir string, port int, offer Offer) !string {
	mut stream := dial_stream('127.0.0.1', port)!
	mut tr := new_transport(Stream(stream), offer, HostCheck{})
	tr.handshake()!
	mut auth := key_auth(dir, 'id_ed25519', '')
	authenticate(mut tr, os.getenv('USERNAME'), mut auth, '')!
	mut conn := new_conn(tr)
	conn.start(Keepalive{ enabled: false })
	defer {
		conn.close()
	}
	out, _ := run_cmd(mut conn, 'echo algo-ok')!
	return out
}

fn test_every_algorithm_against_sshd() {
	dir, port := gate() or { return }
	for k in kex_algorithms {
		out := connect_offering(dir, port, Offer{ kex: [k] }) or {
			assert false, '${k}: ${err}'
			return
		}
		assert out.contains('algo-ok'), k
	}
	for c in cipher_algorithms {
		out := connect_offering(dir, port, Offer{ cipher: [c] }) or {
			assert false, '${c}: ${err}'
			return
		}
		assert out.contains('algo-ok'), c
	}
	for m in mac_algorithms {
		out := connect_offering(dir, port, Offer{
			cipher: ['aes256-ctr']
			mac:    [m]
		}) or {
			assert false, '${m}: ${err}'
			return
		}
		assert out.contains('algo-ok'), m
	}
	for h in ['ssh-ed25519', 'ecdsa-sha2-nistp384', 'rsa-sha2-256', 'rsa-sha2-512'] {
		out := connect_offering(dir, port, Offer{ host_key: [h] }) or {
			assert false, '${h}: ${err}'
			return
		}
		assert out.contains('algo-ok'), h
	}
}

fn test_shell_through_pty_interface() {
	dir, port := gate() or { return }
	mut cfg := base_config(dir, port, key_auth(dir, 'id_ed25519', ''), 'shell')
	cfg.cols = 100
	cfg.rows = 30
	mut s := connect(cfg) or { panic(err) }
	mut got := &ShellSink{}
	mut pty := s.attach(ptyopts(mut got)) or { panic(err) }
	// the prompt, then a command whose output cannot be confused with the echo of its text
	pty.write('echo qu-ad-ra-$((6*7))\r'.bytes()) or { panic(err) }
	mut seen := false
	for _ in 0 .. 500 {
		if got.text().contains('qu-ad-ra-42') {
			seen = true
			break
		}
		time.sleep(20 * time.millisecond)
	}
	assert seen, got.text()
	pty.resize(120, 40) or { panic(err) }
	pty.write('exit 7\r'.bytes()) or { panic(err) }
	mut exited := false
	for _ in 0 .. 500 {
		if got.done.try_wait() {
			exited = true
			break
		}
		time.sleep(20 * time.millisecond)
	}
	assert exited
	assert got.code == 7
	in_, out_ := s.bytes_in_out()
	assert in_ > 0 && out_ > 0
	pty.close()
}
