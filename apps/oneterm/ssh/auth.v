// User authentication (RFC 4252, RFC 4256, RFC 8308) with the behaviour of specs/23.
module ssh

const msg_userauth_request = u8(50)
const msg_userauth_failure = u8(51)
const msg_userauth_success = u8(52)
const msg_userauth_banner = u8(53)
const msg_userauth_info_request = u8(60)
const msg_userauth_info_response = u8(61)

const max_agent_identities = 6
const max_kbd_prompts = 100 // per round; the OpenSSH client allows as many

struct AuthOutcome {
	success   bool
	remaining []string
	partial   bool
}

fn auth_err(message string) IError {
	return ConnectError{
		phase:   .authentication
		message: message
	}
}

// failure_message names the methods the server still accepts.
fn failure_message(o AuthOutcome) string {
	mut m := 'rejected by the server'
	if o.partial {
		m += ' (the server accepted this method but requires another one)'
	}
	if o.remaining.len > 0 {
		m += '; the server accepts: ' + o.remaining.join(', ')
	}
	return m
}

fn userauth_writer(user string, method string) Writer {
	mut w := new_writer(256)
	w.put_u8(msg_userauth_request)
	w.put_str(user)
	w.put_str('ssh-connection')
	w.put_str(method)
	return w
}

// read_auth_reply skips banners and returns the next packet.
fn read_auth_reply(mut t Transport) ![]u8 {
	for {
		pkt := t.next_packet()!
		if pkt[0] != msg_userauth_banner {
			return pkt
		}
	}
	return error('unreachable')
}

fn parse_outcome(pkt []u8) !AuthOutcome {
	if pkt[0] == msg_userauth_success {
		return AuthOutcome{
			success: true
		}
	}
	if pkt[0] != msg_userauth_failure {
		return error('unexpected message ${pkt[0]} during authentication')
	}
	mut r := new_reader(pkt[1..])
	remaining := r.get_names()!
	partial := r.get_bool()!
	return AuthOutcome{false, remaining, partial}
}

fn authenticate(mut t Transport, user string, mut method AuthMethod, agent_pipe string) ! {
	mut w := new_writer(32)
	w.put_u8(msg_service_request)
	w.put_str('ssh-userauth')
	t.send(w.buf)!
	accept := t.next_packet()!
	if accept[0] != msg_service_accept {
		return error('the server refused the authentication service')
	}
	mut outcome := AuthOutcome{}
	match mut method {
		NoAuth {
			t.send(userauth_writer(user, 'none').buf)!
			outcome = parse_outcome(read_auth_reply(mut t)!)!
		}
		PasswordAuth {
			outcome = auth_password(mut t, user, method.password.bytes)!
			method.password.wipe()
		}
		KeyAuth {
			mut key := load_private_key(method.key_path, method.passphrase)!
			if mut p := method.passphrase {
				p.wipe()
			}
			alg := pubkey_alg(mut t, key.kind)
			outcome = auth_publickey_sign(mut t, user, alg, key.public_blob, key)!
			key.wipe()
		}
		AgentAuth {
			outcome = auth_agent(mut t, user, agent_pipe)!
		}
	}

	if !outcome.success {
		return auth_err(failure_message(outcome))
	}
}

fn auth_password(mut t Transport, user string, password []u8) !AuthOutcome {
	mut w := userauth_writer(user, 'password')
	w.put_bool(false)
	w.put_bytes(password)
	t.send(w.buf)!
	pkt := read_auth_reply(mut t)!
	if pkt[0] == msg_userauth_info_request {
		return AuthOutcome{} // SSH_MSG_USERAUTH_PASSWD_CHANGEREQ: treated as a rejection
	}
	first := parse_outcome(pkt)!
	if first.success || 'keyboard-interactive' !in first.remaining {
		return first
	}
	return auth_keyboard(mut t, user, password)
}

// auth_keyboard answers a single round of prompts with the password; prompts that echo or a
// second round are clear errors, not guesses.
fn auth_keyboard(mut t Transport, user string, password []u8) !AuthOutcome {
	mut w := userauth_writer(user, 'keyboard-interactive')
	w.put_str('')
	w.put_str('')
	t.send(w.buf)!
	pkt := read_auth_reply(mut t)!
	if pkt[0] != msg_userauth_info_request {
		return parse_outcome(pkt)
	}
	mut r := new_reader(pkt[1..])
	r.get_str()! // name
	r.get_str()! // instruction
	r.get_str()! // language
	count := int(r.get_u32()!)
	if count < 0 || count > max_kbd_prompts { // the answer repeats the password per prompt
		return auth_err('keyboard-interactive authentication sent ${count} prompts')
	}
	for _ in 0 .. count {
		r.get_str()! // prompt
		if r.get_bool()! {
			return auth_err('keyboard-interactive authentication asked for input other than a password; interactive prompts are not supported')
		}
	}
	mut resp := new_writer(256)
	resp.put_u8(msg_userauth_info_response)
	resp.put_u32(u32(count))
	for _ in 0 .. count {
		resp.put_bytes(password)
	}
	t.send(resp.buf)!
	reply := read_auth_reply(mut t)!
	if reply[0] == msg_userauth_info_request {
		return auth_err('keyboard-interactive authentication requested a second round of prompts; interactive prompts are not supported')
	}
	return parse_outcome(reply)
}

// pubkey_alg picks the signature algorithm: RSA follows server-sig-algs, never SHA-1.
fn pubkey_alg(mut t Transport, kind string) string {
	if kind != 'ssh-rsa' {
		return kind
	}
	t.send_mu.@lock()
	defer {
		t.send_mu.unlock()
	}
	if t.has_sig {
		if 'rsa-sha2-512' in t.sig_algs {
			return 'rsa-sha2-512'
		}
		if 'rsa-sha2-256' in t.sig_algs {
			return 'rsa-sha2-256'
		}
	}
	return 'rsa-sha2-512'
}

// signature_data is what a user key signs (RFC 4252 section 7).
fn signature_data(t &Transport, user string, alg string, blob []u8) []u8 {
	mut d := new_writer(blob.len + 256)
	d.put_bytes(t.session_id)
	d.put_u8(msg_userauth_request)
	d.put_str(user)
	d.put_str('ssh-connection')
	d.put_str('publickey')
	d.put_bool(true)
	d.put_str(alg)
	d.put_bytes(blob)
	return d.buf
}

fn auth_publickey_sign(mut t Transport, user string, alg string, blob []u8, key PrivateKey) !AuthOutcome {
	sig := key.sign(alg, signature_data(t, user, alg, blob))!
	return send_pubkey(mut t, user, alg, blob, sig)
}

fn send_pubkey(mut t Transport, user string, alg string, blob []u8, sig []u8) !AuthOutcome {
	mut w := userauth_writer(user, 'publickey')
	w.put_bool(true)
	w.put_str(alg)
	w.put_bytes(blob)
	w.put_bytes(sig)
	t.send(w.buf)!
	return parse_outcome(read_auth_reply(mut t)!)
}

fn auth_agent(mut t Transport, user string, pipe string) !AuthOutcome {
	mut link := connect_agent(pipe)!
	defer {
		link.close()
	}
	return auth_with_agent(mut t, user, mut link)
}

// auth_with_agent offers identities in the agent's order, at most six, stopping when the
// server no longer accepts publickey.
fn auth_with_agent(mut t Transport, user string, mut link AgentLink) !AuthOutcome {
	identities := agent_identities(mut link) or {
		return auth_err('SSH agent did not list its identities: ${err}')
	}
	if identities.len == 0 {
		return auth_err('the SSH agent holds no identities (add one with ssh-add)')
	}
	mut last := AuthOutcome{}
	mut offered := 0
	for id in identities {
		if offered == max_agent_identities {
			break
		}
		key := parse_host_key(id.blob) or { continue }
		alg := pubkey_alg(mut t, key.kind)
		flags := if alg == 'rsa-sha2-256' {
			agent_flag_sha256
		} else if alg == 'rsa-sha2-512' {
			agent_flag_sha512
		} else {
			u32(0)
		}
		offered++
		sig := agent_sign(mut link, id.blob, signature_data(t, user, alg, id.blob), flags) or {
			return auth_err('SSH agent signing failed: ${err}')
		}
		last = send_pubkey(mut t, user, alg, id.blob, sig)!
		if last.success {
			return last
		}
		if 'publickey' !in last.remaining {
			break
		}
	}
	if offered == 0 {
		return auth_err('the SSH agent holds no usable identities')
	}
	return auth_err('${failure_message(last)}; none of the ${offered} agent ${if offered == 1 {
		'identity'
	} else {
		'identities'
	}} offered was accepted')
}
