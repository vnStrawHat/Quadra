// URL policy: the one gate every terminal-controlled link passes through (specs/22, Links).
// A program can embed a local-launching scheme, a file link, or a link whose label names one
// site and opens another.
module term

const max_url_len = 256 * 1024

const allowed_schemes = ['https']
const confirm_schemes = ['http']

pub enum Verdict {
	allow
	confirm
	deny
}

// TargetDecision is what to do with a link; `why` is the reason for the log.
pub struct TargetDecision {
pub:
	verdict Verdict
	why     string
}

// ParsedUrl is the three fields the policy needs.
struct ParsedUrl {
	scheme   string
	has_cred bool
	has_port bool
	port     int
}

// validate_target decides on a target URL alone.
pub fn validate_target(target string) TargetDecision {
	if target.len > max_url_len {
		return TargetDecision{.deny, 'TooLong(${target.len})'}
	}
	if has_control_chars(target) {
		return TargetDecision{.deny, 'ControlCharacters'}
	}
	parsed := parse_url(target) or { return TargetDecision{.deny, 'InvalidUrl("${target}")'} }
	scheme := parsed.scheme.to_lower()
	if scheme in allowed_schemes {
		if parsed.has_cred {
			return TargetDecision{.deny, 'HasCredentials'}
		}
		if parsed.has_port && parsed.port != default_port(scheme) {
			return TargetDecision{.confirm, 'NonDefaultPort'}
		}
		return TargetDecision{.allow, ''}
	}
	if scheme in confirm_schemes {
		if parsed.has_cred {
			return TargetDecision{.deny, 'HasCredentials'}
		}
		return TargetDecision{.confirm, 'RequiresConfirmation'}
	}
	return TargetDecision{.deny, 'SchemeNotAllowed("${parsed.scheme}")'}
}

// validate_target_with_display also compares an OSC 8 label with its target: a URL-shaped
// label naming another host never opens silently. `display` is '' when there is no label.
pub fn validate_target_with_display(target string, display string) TargetDecision {
	base := validate_target(target)
	if base.verdict == .allow && display != '' && !display_matches_target(display, target) {
		return TargetDecision{.confirm, 'DisplayTargetMismatch'}
	}
	return base
}

// display_matches_target is true when a label is empty, descriptive, or names the target's host.
pub fn display_matches_target(display_ string, target string) bool {
	display := display_.trim_space()
	if display == '' || display.to_lower() == target.trim_space().to_lower() {
		return true
	}
	host := url_like_host(display) or { return true }
	target_host := url_like_host(target) or { return false }
	return host == target_host
}

fn is_scheme_char(c u8, allow_dot bool) bool {
	return c.is_letter() || c.is_digit() || c == `+` || c == `-` || (allow_dot && c == `.`)
}

fn valid_scheme(s string, allow_dot bool) bool {
	if s == '' || !s[0].is_letter() {
		return false
	}
	for c in s {
		if !is_scheme_char(c, allow_dot) {
			return false
		}
	}
	return true
}

fn all_digits(s string) bool {
	if s == '' {
		return false
	}
	for c in s {
		if !c.is_digit() {
			return false
		}
	}
	return true
}

// authority_end is the end of the authority: the first `/`, `?` or `#`.
fn authority_end(s string) int {
	for i, c in s {
		if c == `/` || c == `?` || c == `#` {
			return i
		}
	}
	return s.len
}

// url_like_host is the lower-cased host without `www.` or a port; none when the text is not
// URL-shaped (no `scheme://` and no `host.tld` first segment).
fn url_like_host(text_ string) ?string {
	text := text_.trim_space()
	has_scheme := text.contains('://')
	mut after := text
	if has_scheme {
		idx := text.index('://') or { 0 }
		if !valid_scheme(text[..idx], true) {
			return none
		}
		after = text[idx + 3..]
	} else if text.contains(':') {
		idx := text.index(':') or { 0 }
		scheme, rest := text[..idx], text[idx + 1..]
		first := rest[..authority_end(rest)]
		// `mailto:x@y`, `data:...`: a scheme without an authority has no host; `host:port` does.
		if valid_scheme(scheme, false) && !rest.starts_with('/') && !all_digits(first) {
			return none
		}
	}
	authority := after[..authority_end(after)]
	host_port := authority.all_after_last('@')
	mut host := host_port
	if host_port.starts_with('[') { // IPv6 literal
		rest := host_port[1..]
		host = rest.all_before(']')
	} else if host_port.contains(':') {
		colon := host_port.last_index(':') or { 0 }
		if all_digits(host_port[colon + 1..]) || colon + 1 == host_port.len {
			host = host_port[..colon]
		}
	}
	mut h := host.trim_right('.').to_lower()
	if h.starts_with('www.') {
		h = h[4..]
	}
	if h == '' || (!has_scheme && !h.contains('.')) || has_host_junk(h) {
		return none
	}
	return h
}

fn has_host_junk(h string) bool {
	for r in h.runes() {
		if is_white(r) || r == `/` || r == `\\` || r == `"` || r == `'` {
			return true
		}
	}
	return false
}

// parse_url extracts the scheme, the credentials flag and the port.
fn parse_url(input string) ?ParsedUrl {
	colon := input.index(':') or { return none }
	scheme := input[..colon]
	if !valid_scheme(scheme, true) {
		return none
	}
	after := input[colon + 1..]
	if !after.starts_with('//') {
		return ParsedUrl{
			scheme: scheme
		}
	}
	auth := after[2..]
	authority := auth[..authority_end(auth)]
	has_cred := authority.contains('@')
	host_part := authority.all_after_last('@')
	// `[::1]:8443`: the port can only follow the closing bracket.
	mut port_source := host_part
	if host_part.starts_with('[') {
		close := host_part.index(']') or { host_part.len - 1 }
		port_source = host_part[close + 1..]
	}
	mut has_port := false
	mut port := 0
	if port_source.contains(':') {
		idx := port_source.last_index(':') or { 0 }
		s := port_source[idx + 1..]
		if all_digits(s) && s.len <= 5 && s.int() <= 65535 {
			has_port, port = true, s.int()
		}
	}
	return ParsedUrl{scheme, has_cred, has_port, port}
}

// has_control_chars rejects C0, DEL, C1 and bidi controls, which can reorder a visible address.
fn has_control_chars(s string) bool {
	for r in s.runes() {
		c := u32(r)
		if c < 0x20 || c == 0x7f || (c >= 0x80 && c <= 0x9f) || is_bidi_control(c) {
			return true
		}
	}
	return false
}

fn is_bidi_control(c u32) bool {
	return (c >= 0x202a && c <= 0x202e) || (c >= 0x2066 && c <= 0x2069)
		|| c == 0x200e || c == 0x200f
}

fn default_port(scheme string) int {
	return match scheme {
		'https' { 443 }
		'http' { 80 }
		'ftp' { 21 }
		else { 0 }
	}
}

// is_white is char::is_whitespace of the original.
fn is_white(r rune) bool {
	c := u32(r)
	return c == 32 || (c >= 9 && c <= 13) || c == 0x85 || c == 0xa0 || c == 0x1680
		|| (c >= 0x2000 && c <= 0x200a) || c == 0x2028 || c == 0x2029 || c == 0x202f
		|| c == 0x205f || c == 0x3000
}

// url_confirmation_text is the dialog's question: the label against the real target, so a
// mismatch can be seen.
pub fn url_confirmation_text(url string, display string) string {
	label := display.trim_space()
	if label != '' && label.to_lower() != url.to_lower() {
		return 'This link is labelled "${display}" but opens ${url}. Open it?'
	}
	return 'Open ${url} in your browser?'
}
