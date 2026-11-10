// Secret-classification heuristics: sensitive flag names, credential-shaped tokens,
// URL userinfo.
module completion

import math

// Flag names whose value is a secret. Single-letter flags are deliberately absent: `-p`
// means "password" for mysql and "parents" for mkdir; see short_secret_flag.
const secret_flag_vocab = ['password', 'passwd', 'pwd', 'pass', 'secret', 'token', 'api-key',
	'apikey', 'access-key', 'secret-key', 'auth', 'authorization', 'bearer', 'credential',
	'private-key', 'client-secret', 'session-token', 'otp', 'passphrase']

// Commands whose `-p` takes a password.
const password_p_commands = ['mysql', 'mysqladmin', 'mysqldump', 'mysqlimport', 'mariadb',
	'mariadb-dump', 'sshpass', 'az']

// Container CLIs whose `login` takes `-p <password>`; elsewhere `-p` publishes a port.
const container_login_commands = ['docker', 'podman', 'nerdctl']

// Credential prefixes, each followed by at least 8 chars.
const secret_prefixes = ['ghp_', 'gho_', 'ghs_', 'ghu_', 'ghr_', 'github_pat_', 'sk-', 'xox']

// normalize_flag strips the triggers, lowercases and maps `_` to `-`.
fn normalize_flag(name string) string {
	mut i := 0
	for i < name.len && (name[i] == `-` || name[i] == `/`) {
		i++
	}
	return name[i..].to_lower().replace('_', '-')
}

fn is_secret_flag(name string) bool {
	n := normalize_flag(name)
	return n in secret_flag_vocab || n.ends_with('-password') || n.ends_with('-passwd')
		|| n.ends_with('-secret') || n.ends_with('-token')
}

// short_secret_flag returns the letter of the short secret flag of `command`, 0 for none.
fn short_secret_flag(command string, subcommand string) u8 {
	if command in password_p_commands {
		return `p`
	}
	if command in container_login_commands && subcommand == 'login' {
		return `p`
	}
	if command == 'curl' {
		return `u`
	}
	return 0
}

fn is_command_secret_long_flag(command string, name string) bool {
	return command == 'curl' && normalize_flag(name) == 'user'
}

// command_name normalizes a command token: basename, lowercase, no `.exe`.
fn command_name(token string) string {
	mut start := 0
	for i, c in token {
		if c == `/` || c == `\\` {
			start = i + 1
		}
	}
	base := token[start..].to_lower()
	return if base.ends_with('.exe') { base[..base.len - 4] } else { base }
}

fn is_secret_key(key string) bool {
	k := key.to_lower()
	if k.replace('_', '-') in secret_flag_vocab {
		return true
	}
	return k.ends_with('_token') || k.ends_with('_key') || k.ends_with('_secret')
		|| k.ends_with('_password') || k.contains('secret') || k.contains('password')
		|| k.contains('token')
}

// strip_url_userinfo turns `scheme://user:pass@host/...` into `scheme://host/...`.
fn strip_url_userinfo(tok string) ?string {
	scheme_end := tok.index('://') or { return none }
	after := tok[scheme_end + 3..]
	at := after.index_u8(`@`)
	if at < 0 || after[..at].index_u8(`:`) < 0 {
		return none
	}
	return tok[..scheme_end] + '://' + after[at + 1..]
}

// looks_like_secret: does a standalone token look like a credential?
fn looks_like_secret(tok string) bool {
	t := tok.trim('"\'')
	if t.len == 0 {
		return false
	}
	if t.starts_with('AKIA') && t.len >= 16 && t[4..].bytes().all(it.is_alnum()) {
		return true
	}
	for p in secret_prefixes {
		if t.starts_with(p) && t.len >= p.len + 8 {
			return true
		}
	}
	if t.contains('-----BEGIN') {
		return true
	}
	low := t.to_lower()
	if low.contains('bearer ') || low.starts_with('authorization') {
		return true
	}
	return is_jwt(t) || is_high_entropy(t)
}

fn is_jwt(t string) bool {
	parts := t.split('.')
	if parts.len != 3 || !parts[0].starts_with('eyJ') {
		return false
	}
	for p in parts {
		if p.len < 4 || !p.bytes().all(is_base64url_char(it)) {
			return false
		}
	}
	return true
}

fn is_base64url_char(c u8) bool {
	return c.is_alnum() || c == `-` || c == `_` || c == `+` || c == `/` || c == `=`
}

// is_high_entropy: a long token, mostly base64 alphabet, letters and digits, no separators.
fn is_high_entropy(t string) bool {
	if t.len < 20 || t.contains('/') || t.contains('\\') || t.contains('://') {
		return false
	}
	mut alnum := 0
	mut has_digit := false
	mut has_alpha := false
	for c in t {
		if c.is_alnum() || c == `+` || c == `/` || c == `_` || c == `-` {
			alnum++
		}
		has_digit = has_digit || c.is_digit()
		has_alpha = has_alpha || c.is_letter()
	}
	return alnum * 100 / t.len >= 90 && has_digit && has_alpha && shannon_entropy(t) >= 3.5
}

fn shannon_entropy(s string) f64 {
	runes := s.runes()
	if runes.len == 0 {
		return 0.0
	}
	mut counts := map[rune]int{}
	for r in runes {
		counts[r]++
	}
	mut entropy := 0.0
	len := f64(runes.len)
	for _, count in counts {
		p := f64(count) / len
		entropy -= p * math.log2(p)
	}
	return entropy
}
