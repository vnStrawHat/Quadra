module sessionui

fn test_user_alone_leaves_host_and_port_to_the_caller() {
	p := parse_user_host_port('  root  ')!
	assert p.user == 'root' && p.host == none && p.port == none
	assert parse_user_host_port('')!.user == ''
}

fn test_user_host_and_optional_port() {
	p := parse_user_host_port('root@example.test:2222')!
	assert p.user == 'root' && p.host or { '' } == 'example.test' && p.port or { 0 } == 2222
	assert parse_user_host_port('root@10.0.0.1')!.port == none
}

fn test_ipv6_hosts_are_recognised() {
	a := parse_user_host_port('root@[2001:db8::1]:2200')!
	assert a.host or { '' } == '2001:db8::1' && a.port or { 0 } == 2200
	b := parse_user_host_port('root@[::1]')!
	assert b.host or { '' } == '::1' && b.port == none
	c := parse_user_host_port('root@fe80::1')!
	assert c.host or { '' } == 'fe80::1' && c.port == none
	parse_user_host_port('root@[::1') or {
		assert err.msg().contains('brackets')
		return
	}
	assert false
}

fn test_errors_name_the_problem() {
	parse_user_host_port('@host') or {
		assert err.msg() == "Username is required before the '@'."
		parse_user_host_port('root@') or {
			assert err.msg() == "Host is required after the '@'."
			parse_user_host_port('root@host:abc') or {
				assert err.msg().contains('65535')
				parse_port('0') or {
					assert err.msg().contains('between 1')
					assert parse_port(' 22 ')! == 22
					return
				}
				assert false
				return
			}
			assert false
			return
		}
		assert false
		return
	}
	assert false
}

fn test_explicit_fields_take_precedence_over_the_username_string() {
	assert resolve_target('root@parsed.test:2200', 'typed.test', '22')!.label() == 'root@typed.test:22'
	assert resolve_target('root@parsed.test:2200', '', '')!.label() == 'root@parsed.test:2200'
	assert resolve_target('root@parsed.test', '', '')!.port == 22
	assert resolve_target('root', 'typed.test', '')!.host == 'typed.test'
}

fn test_missing_or_invalid_targets_are_reported() {
	resolve_target('root', '', '') or {
		assert err.msg() == 'Host is required.'
		resolve_target('', 'typed.test', '') or {
			assert err.msg() == 'Username is required.'
			return
		}
		assert false
		return
	}
	assert false
}
