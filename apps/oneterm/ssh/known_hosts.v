// OpenSSH known_hosts: lookup, the verdict table of specs/23, and appending a new key.
module ssh

import crypto.hmac
import crypto.sha1
import encoding.base64
import os

const hashed_magic = '|1|'

struct KnownKey {
	kind    string
	blob    []u8
	revoked bool
}

// host_port is the name known_hosts files use for an address.
fn host_port(host string, port int) string {
	if port == 22 {
		return host
	}
	return '[${host}]:${port}'
}

// glob_match matches * and ? with one backtrack point, in O(pattern * name): the recursive
// form was exponential in the stars of a hostile pattern.
fn glob_match(pattern string, name string) bool {
	mut p, mut n := 0, 0
	mut star, mut mark := -1, 0
	for n < name.len {
		if p < pattern.len && (pattern[p] == `?` || pattern[p] == name[n]) {
			p++
			n++
		} else if p < pattern.len && pattern[p] == `*` {
			star, mark = p, n
			p++
		} else if star >= 0 {
			p = star + 1
			mark++
			n = mark
		} else {
			return false
		}
	}
	for p < pattern.len && pattern[p] == `*` {
		p++
	}
	return p == pattern.len
}

fn hashed_matches(pattern string, name string) bool {
	parts := pattern.split('|')
	if parts.len != 4 {
		return false
	}
	salt := b64_decode(parts[2])
	want := b64_decode(parts[3])
	if salt.len == 0 || want.len != 20 {
		return false
	}
	return hmac.equal(hmac.new(salt, name.bytes(), sha1.sum, sha1.block_size), want)
}

// patterns_match applies comma-separated patterns, negations and hashed names to a host name.
fn patterns_match(patterns string, name string) bool {
	if patterns.starts_with(hashed_magic) {
		return hashed_matches(patterns, name)
	}
	mut matched := false
	for pat in patterns.split(',') {
		if pat.starts_with('!') {
			if glob_match(pat[1..], name) {
				return false
			}
		} else if glob_match(pat, name) {
			matched = true
		}
	}
	return matched
}

// known_keys returns the entries of a file recorded for host_name. An unreadable file, or a
// matching entry that cannot be parsed, is an error (fail closed). A missing file has no entries.
fn known_keys(path string, host_name string) ![]KnownKey {
	if !os.exists(path) {
		return []
	}
	text := os.read_file(path) or { return error('known_hosts is not readable: ${err}') }
	mut out := []KnownKey{}
	for raw in text.split_into_lines() {
		line := raw.trim_space()
		if line == '' || line.starts_with('#') {
			continue
		}
		mut fields := line.fields()
		mut revoked := false
		if fields[0].starts_with('@') {
			if fields[0] == '@revoked' {
				revoked = true
			}
			if fields[0] == '@cert-authority' {
				continue
			}
			fields = fields[1..].clone()
		}
		if fields.len < 3 || !patterns_match(fields[0], host_name) {
			continue
		}
		blob := b64_decode(fields[2])
		key := parse_host_key(blob) or {
			return error('known_hosts entry for ${host_name} is malformed: ${err}')
		}
		if key.kind != fields[1] {
			return error('known_hosts entry for ${host_name} is malformed')
		}
		out << KnownKey{key.kind, blob, revoked}
	}
	return out
}

// preferred_algorithms moves algorithms already recorded for the host to the front, keeping
// the default order within both groups; RSA entries match every RSA signature flavour.
fn preferred_algorithms(known []KnownKey) []string {
	mut first := []string{}
	mut rest := []string{}
	for alg in host_key_algorithms {
		mut hit := false
		for k in known {
			if k.kind == alg || (k.kind == 'ssh-rsa' && is_rsa_alg(alg)) {
				hit = true
				break
			}
		}
		if hit {
			first << alg
		} else {
			rest << alg
		}
	}
	first << rest
	return first
}

// check_host_key decides whether the presented key may be used for host:port.
fn check_host_key(path string, host string, port int, policy HostKeyPolicy, key HostKey) ! {
	name := host_port(host, port)
	entries := known_keys(path, name)!
	fp := key.fingerprint()
	for k in entries {
		if k.blob == key.blob {
			if k.revoked {
				return HostKeyChanged{
					host:        host
					port:        port
					fingerprint: fp
				}
			}
			return
		}
	}
	if entries.len > 0 {
		return HostKeyChanged{
			host:        host
			port:        port
			fingerprint: fp
		}
	}
	if policy.fingerprint != '' && policy.fingerprint == fp {
		learn_known_host(path, name, key)!
		return
	}
	return HostKeyUnknown{
		host:        host
		port:        port
		algorithm:   key.kind
		fingerprint: fp
	}
}

// learn_known_host appends a line, creating the folder if needed.
fn learn_known_host(path string, name string, key HostKey) ! {
	dir := os.dir(path)
	if dir != '' && !os.exists(dir) {
		os.mkdir_all(dir)!
	}
	mut prefix := ''
	if os.exists(path) {
		existing := os.read_file(path)!
		if existing.len > 0 && !existing.ends_with('\n') {
			prefix = '\n'
		}
	}
	mut f := os.open_append(path)!
	defer {
		f.close()
	}
	f.write_string('${prefix}${name} ${key.kind} ${base64.encode(key.blob)}\n')!
}
