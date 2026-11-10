// The target policy, class by class, as in the original.
module term

fn test_https_opens_http_confirms() {
	assert validate_target('https://example.com/path').verdict == .allow
	assert validate_target('HtTpS://Example.COM/Path').verdict == .allow
	assert validate_target('https://例え.テスト/path').verdict == .allow
	http := validate_target('http://example.com/path')
	assert http.verdict == .confirm && http.why == 'RequiresConfirmation'
}

fn test_non_default_ports_confirm() {
	d := validate_target('https://example.com:8443/path')
	assert d.verdict == .confirm && d.why == 'NonDefaultPort'
	assert validate_target('https://example.com:443/path').verdict == .allow
	assert validate_target('https://[::1]/').verdict == .allow
	assert validate_target('https://[2001:db8::1]:443/path').verdict == .allow
	assert validate_target('https://[::1]:8443/').verdict == .confirm
}

fn test_credentials_are_denied() {
	d := validate_target('https://user:secret@example.com/private')
	assert d.verdict == .deny && d.why == 'HasCredentials'
	assert validate_target('http://u@example.com/').verdict == .deny
}

fn test_non_web_schemes_are_denied() {
	for url in ['file:///C:/Windows/System32/cmd.exe', 'custom-app://run?action=delete',
		'ssh://user@host/path', 'mailto:someone@example.com'] {
		d := validate_target(url)
		assert d.verdict == .deny && d.why.starts_with('SchemeNotAllowed'), url
	}
	assert validate_target('not a url').verdict == .deny
}

fn test_controls_and_bidi_are_denied() {
	for url in ['https://example.com/a\nb', 'https://example.com/a\rb', 'https://example.com/a\tb',
		'https://example.com/\u0007control', 'https://example.com/\u2066evil\u2069',
		'https://example.com/\u202eevil', 'https://example.com/\u007f'] {
		d := validate_target(url)
		assert d.verdict == .deny && d.why == 'ControlCharacters', url
	}
	long := 'https://example.com/${'x'.repeat(256 * 1024)}'
	assert validate_target(long).verdict == .deny
}

fn test_a_label_naming_another_host_confirms() {
	for pair in [['click here', 'https://example.com/x'], ['Release notes', 'https://example.com/x'],
		['', 'https://example.com/x'], ['example.com', 'https://example.com/x'],
		['EXAMPLE.com/path', 'https://example.com/other'], ['www.example.com', 'https://example.com'],
		['https://example.com:443/', 'https://example.com/'],
		['[::1]/', 'https://[::1]:8443/'], ['https://user@example.com', 'https://example.com']] {
		assert display_matches_target(pair[0], pair[1]), pair.str()
	}
	for pair in [['https://good.com', 'https://evil.com'],
		['good.com/login', 'https://evil.com/login'], ['www.good.com', 'https://evil.com'],
		['https://example.com', 'mailto:someone@example.com']] {
		assert !display_matches_target(pair[0], pair[1]), pair.str()
	}
	mismatch := validate_target_with_display('https://evil.com/login', 'good.com/login')
	assert mismatch.verdict == .confirm && mismatch.why == 'DisplayTargetMismatch'
	assert validate_target_with_display('https://example.com/x', 'click here').verdict == .allow
	// A denied or confirmed target stays what it was, label or no label.
	assert validate_target_with_display('file:///x', 'x').verdict == .deny
}

fn test_the_dialog_text() {
	assert url_confirmation_text('http://x.io', '') == 'Open http://x.io in your browser?'
	assert url_confirmation_text('https://evil.com', 'good.com') == 'This link is labelled "good.com" but opens https://evil.com. Open it?'
	assert url_confirmation_text('http://x.io', 'HTTP://X.IO') == 'Open http://x.io in your browser?'
}
