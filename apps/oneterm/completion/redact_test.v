module completion

fn test_drops_value_after_secret_flag() {
	assert redact('az login --password S3cr3t!') == 'az login --password'
}

fn test_drops_inline_secret_value_forms() {
	assert redact('login --password=S3cr3t!') == 'login --password'
	assert redact('login /PASSWORD:S3cr3t!') == 'login /PASSWORD'
	assert redact('mysql -p S3cr3t!') == 'mysql -p'
}

fn test_short_secret_flags_are_per_command_and_detect_attached_values() {
	assert redact('mysql -pSECRET db') == 'mysql -p db'
	assert redact('mysql -p SECRET db') == 'mysql -p db'
	assert redact('mysql -p=SECRET db') == 'mysql -p db'
	assert redact('sudo mysql -pSECRET db') == 'sudo mysql -p db'
	assert redact(r'C:\Tools\MySQL.EXE -pSECRET db') == r'C:\Tools\MySQL.EXE -p db'
	assert redact('curl -uadmin:pw https://x') == 'curl -u https://x'
	assert redact('curl --user admin:pw https://x') == 'curl --user https://x'
	assert redact('docker login -p secret') == 'docker login -p'
	assert redact('docker login -psecret registry') == 'docker login -p registry'
	assert contains_secret('mysql -pSECRET db')
}

fn test_ordinary_p_flags_keep_their_argument() {
	assert redact('mkdir -p dir') == 'mkdir -p dir'
	assert redact('docker run -p 8080:80 img') == 'docker run -p 8080:80 img'
	assert redact('ssh -p 22 host') == 'ssh -p 22 host'
	assert redact('mysql -u root db') == 'mysql -u root db'
	assert !contains_secret('mkdir -p dir')
}

fn test_compound_secret_long_flags_drop_their_value() {
	assert redact('wget --http-password=x url') == 'wget --http-password url'
	assert redact('cli --access-token abc def') == 'cli --access-token def'
	assert redact('scp -i ~/.ssh/key f h:') == 'scp -i ~/.ssh/key f h:'
}

fn test_drops_secret_env_assignment() {
	assert redact('AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI aws s3 ls') == 'aws s3 ls'
}

fn test_keeps_ordinary_assignment() {
	assert redact('RUST_LOG=debug cargo test') == 'RUST_LOG=debug cargo test'
}

fn test_drops_standalone_credential_tokens() {
	assert !redact('echo ghp_0123456789abcdefghij').contains('ghp_')
	assert !redact('echo sk-0123456789abcdef0123').contains('sk-')
	assert !redact('echo AKIAIOSFODNN7EXAMPLE').contains('AKIA')
	assert !redact('echo eyJhbGciOi.eyJzdWIiOi.SflKxwRJSM').contains('eyJ')
}

fn test_strips_url_userinfo_but_keeps_host() {
	assert redact('psql postgres://user:pass@db.example.com/app') == 'psql postgres://db.example.com/app'
}

fn test_does_not_over_redact_normal_command() {
	assert redact('dir /Q') == 'dir /Q'
	assert redact('grep --color -n foo file.txt') == 'grep --color -n foo file.txt'
	assert redact('cargo build --release --output file.txt') == 'cargo build --release --output file.txt'
}

fn test_header_bearer_value_dropped_flag_kept() {
	redacted := redact('curl -H Authorization:Bearer-abcdefgh')
	assert redacted.starts_with('curl -H')
	assert !redacted.contains('abcdefgh')
}

fn test_suggestion_time_guard_detects_injected_secret() {
	assert contains_secret('deploy --token ghp_0123456789abcdefghij')
	assert contains_secret('echo AKIAIOSFODNN7EXAMPLE')
	assert !contains_secret('git commit -m message')
	assert !contains_secret('dir /Q')
}

// Every path into the store goes through capture: a secret must never be recorded.
fn test_a_secret_is_never_stored() {
	c := new_controller('bash', Config{})
	mut h := new_history(10)
	for line in [
		'az login --password S3cr3t!',
		'mysql -pS3cr3t db',
		'AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI aws s3 ls',
		'echo ghp_0123456789abcdefghij',
		'psql postgres://user:hunter2@db/app',
	] {
		c.capture(line, 1000, mut h)
	}
	assert h.entries(.unix).len > 0
	for rec in h.entries(.unix) {
		assert !rec.line.contains('S3cr3t')
		assert !rec.line.contains('wJalr')
		assert !rec.line.contains('ghp_')
		assert !rec.line.contains('hunter2')
	}
}
