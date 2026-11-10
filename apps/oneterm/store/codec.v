// ssh_session.json: JSON to Session and back, keeping the Rust schema v2 field for field.
module store

import x.json2

const session_schema = u32(2)
const sessions_doc_name = 'ssh_session.json'
const next_id_field = 'next_session_id'
const sessions_field = 'sessions'
const sealed_field = 'sealed_password'

fn bad_field(name string) InvalidData {
	return InvalidData{
		reason: '${sessions_doc_name}: bad field ${name}'
	}
}

// check_text_fields refuses optional string fields of the wrong type, as serde does.
fn check_text_fields(fields map[string]json2.Any, keys []string) ! {
	for key in keys {
		value := fields[key] or { continue }
		if !is_null(value) && string_of(value) == none {
			return bad_field(key)
		}
	}
}

// opt_text is an optional string field: absent or null is none.
fn opt_text(fields map[string]json2.Any, key string) ?string {
	value := fields[key] or { return none }
	return string_of(value)
}

fn req_text(fields map[string]json2.Any, key string) !string {
	value := fields[key] or { return bad_field(key) }
	return string_of(value) or { return bad_field(key) }
}

fn port_value(value json2.Any, key string) !int {
	number := u64_of(value) or { return bad_field(key) }
	if number > 65535 {
		return bad_field(key)
	}
	return int(number)
}

fn is_ip_address(text string) bool {
	if text.contains(':') {
		return text.len >= 2 && text.all_before('%').to_lower().contains_only('0123456789abcdef:.')
	}
	parts := text.split('.')
	if parts.len != 4 {
		return false
	}
	for p in parts {
		if p == '' || p.len > 3 || !p.contains_only('0123456789') || p.int() > 255 {
			return false
		}
	}
	return true
}

fn decode_forward(item json2.Any) !PortForward {
	fields := object_of(item) or { return bad_field('port_forwards') }
	kind_text := req_text(fields, 'kind')!
	kind := match kind_text {
		'local' { ForwardKind.local }
		'remote' { ForwardKind.remote }
		'dynamic' { ForwardKind.dynamic }
		else { return bad_field('kind') }
	}

	mut out := PortForward{
		kind:      kind
		bind_port: port_value(fields['bind_port'] or { return bad_field('bind_port') }, 'bind_port')!
	}
	check_text_fields(fields, ['bind', 'bind_host'])!
	if kind == .remote {
		out.bind = opt_text(fields, 'bind_host') or { '127.0.0.1' }
	} else {
		out.bind = opt_text(fields, 'bind') or { '127.0.0.1' }
		if !is_ip_address(out.bind) {
			return bad_field('bind')
		}
	}
	if kind != .dynamic {
		out.target_host = req_text(fields, 'target_host')!
		out.target_port = port_value(fields['target_port'] or { return bad_field('target_port') },
			'target_port')!
	}
	return out
}

fn decode_session(fields map[string]json2.Any) !Session {
	mut s := Session{
		label: req_text(fields, 'label')!
		host:  req_text(fields, 'host')!
	}
	if port := fields['port'] {
		s.port = port_value(port, 'port')!
	}
	check_text_fields(fields, ['username', 'key_path', 'color', 'group', 'auth_method', 'logging'])!
	s.username = opt_text(fields, 'username')
	s.key_path = opt_text(fields, 'key_path')
	s.color = opt_text(fields, 'color')
	s.group = opt_text(fields, 'group')
	if text := opt_text(fields, 'auth_method') {
		s.auth_method = match text {
			'password' { AuthMethod.password }
			'private_key' { AuthMethod.private_key }
			'agent' { AuthMethod.agent }
			else { return bad_field('auth_method') }
		}
	}
	if text := opt_text(fields, 'logging') {
		s.logging = match text {
			'inherit' { LoggingOverride.inherit }
			'on' { LoggingOverride.on }
			'off' { LoggingOverride.off }
			else { return bad_field('logging') }
		}
	}
	if jump := fields['jump_host'] {
		if !is_null(jump) {
			s.jump_host = u64_of(jump) or { return bad_field('jump_host') }
		}
	}
	if list := fields['port_forwards'] {
		for item in array_of(list) or { return bad_field('port_forwards') } {
			s.forwards << decode_forward(item)!
		}
	}
	if forwarding := fields['agent_forwarding'] {
		s.agent_forwarding = bool_of(forwarding) or { return bad_field('agent_forwarding') }
	}
	// Lenient on purpose: a non-string sealed value loads as absent, never quarantines the
	// sessions, and any string is kept as an opaque envelope.
	if sealed := fields[sealed_field] {
		s.sealed_password = string_of(sealed)
	}
	return s
}

fn encode_forward(f PortForward) json2.Any {
	mut out := map[string]json2.Any{}
	out['kind'] = json2.Any(f.kind.str())

	out[if f.kind == .remote {
		'bind_host'
	} else {
		'bind'
	}] = json2.Any(f.bind)
	out['bind_port'] = json2.Any(i64(f.bind_port))
	if f.kind != .dynamic {
		out['target_host'] = json2.Any(f.target_host)
		out['target_port'] = json2.Any(i64(f.target_port))
	}
	return json2.Any(out)
}

fn encode_entry(e Entry) json2.Any {
	s := e.session
	mut out := map[string]json2.Any{}
	out['id'] = json2.Any(i64(e.id))
	out['label'] = json2.Any(s.label)
	out['host'] = json2.Any(s.host)
	out['port'] = json2.Any(i64(s.port))
	if username := s.username {
		out['username'] = json2.Any(username)
	}
	if s.auth_method != .password {
		out['auth_method'] = json2.Any(s.auth_method.str())
	}
	if key := s.key_path {
		out['key_path'] = json2.Any(key)
	}
	if color := s.color {
		out['color'] = json2.Any(color)
	}
	if group := s.group {
		out['group'] = json2.Any(group)
	} else {
		out['group'] = null_value()
	}
	if s.logging != .inherit {
		out['logging'] = json2.Any(s.logging.str())
	}
	if jump := s.jump_host {
		out['jump_host'] = json2.Any(i64(jump))
	}
	if s.forwards.len > 0 {
		out['port_forwards'] = json2.Any(s.forwards.map(encode_forward(it)))
	}
	if s.agent_forwarding {
		out['agent_forwarding'] = json2.Any(true)
	}
	if sealed := s.sealed_password {
		out[sealed_field] = json2.Any(sealed)
	}
	return json2.Any(out)
}
