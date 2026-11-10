module highlight

// The scanner's output pinned against the original: every class of a fixed corpus under
// every role and profile hashes to the value the original recorded (specs/24).
const golden = $embed_file('testdata/scan_line_output.golden').to_string()

const bases = [
	'error: something failed',
	'warning: unused variable `x` at src/main.rs:12:5',
	'ok 3 passed; 0 failed; finished in 0.42s',
	'ping 192.168.1.1 from 10.0.0.254 ttl=64 time=1.5ms',
	'Resolved example.com -> 2607:f8b0:4004:80a::200e',
	'link/ether 00:1a:2b:3c:4d:5e brd ff:ff:ff:ff:ff:ff',
	'2026-09-22 10:00:00 INFO server started on :8080',
	'drwxr-xr-x  2 user group 4096 Sep 22 10:00 /var/log/app.log',
	r'-rwsr-sr-t 1 root root 0 Jan 1 C:\Windows\System32\cmd.exe',
	r'curl --url URL -x=type --verbose -H ' + "'Accept: */*' " + r'"quoted text"',
	'value = 0x1F + -42 * 1.5e3 / 89% (a[b]) {c} | d && e || f; g',
	r'$ ls -la ~/projects | grep rs',
	r'user@host:~/src$ cargo build --release',
	r'PS C:\Program Files\App> Get-ChildItem -Recurse',
	r'C:\Users\John Doe\ws>dir /s /b > out.txt',
	r'[user@host ~]$ echo ' + "'unclosed",
	'> quoted mail line with no prompt',
	'',
	'   ',
	r'C:\Users\John Doe\customer\acme\backend\gateway>git commit -m src/main.rs ',
	'the quick brown fox jumps over the lazy dog and keeps on running',
	'2026-09-22 10:00:00 error connect 192.168.1.10 failed warn retry /var/log/app.log 42 info ok 10.0.0.1 debug 7 /usr/bin/thing',
	'\u65e5\u672c\u8a9e error at /etc/hosts \u4e2d\u6587\u6d4b\u8bd5 2026-09-22 \u4e2d\u6587 error 192.168.0.1',
	'\u65e5\u672c\u8a9e2026-09-22 caf\u00e9Mon \u00e900:1a:2b:3c:4d:5e',
]

// variants returns each base as written, and with non-ASCII text before, around and inside.
fn variants(base string) []string {
	chars := base.runes()
	mid := chars[..chars.len / 2].string().len
	return [
		base,
		'\u65e5\u672c\u8a9e ${base}',
		'Ti\u1ebfng Vi\u1ec7t c\u00f3 d\u1ea5u: ${base} l\u1ed7i',
		'🚀 ${base} \u2705 👩\u200d💻',
		'${base[..mid]} \u4e2d\u6587 l\u1ed7i 🎉 ${base[mid..]}',
		'/home/ng\u01b0\u1eddi/t\u1ec7p.rs ${base} \u00e9t\u00e9 200\u00a0ms',
	]
}

fn hash_case(line string) u64 {
	mut sign := -1
	for i, b in line {
		if b == `$` || b == `>` || b == `#` {
			sign = line[..i].runes().len + 2
			break
		}
	}
	mut hash := u64(0xcbf29ce484222325)
	eat := fn (hash u64, b u8) u64 {
		return (hash ^ u64(b)) * 0x00000100000001b3
	}
	for profile in [ShellProfile.unix, .cmd, .powershell, .dumb] {
		cases := [
			[int(RowRole.unmarked), -1],
			[int(RowRole.output), -1],
			[int(RowRole.command), -1],
			[int(RowRole.prompt), -1],
			[int(RowRole.prompt), sign],
		]
		for c in cases {
			role := unsafe { RowRole(c[0]) }
			classes := scan_line(line, profile, role, c[1])
			assert classes.len == line.runes().len, line
			hash = eat(hash, 0xff)
			for b in classes {
				hash = eat(hash, b)
			}
		}
	}
	return hash
}

fn test_classes_match_the_recorded_corpus() {
	mut actual := []string{}
	for b, base in bases {
		for v, line in variants(base) {
			actual << '${b} ${v} ${hash_case(line).hex_full()}'
		}
	}
	expected := golden.trim_space().split_into_lines()
	assert actual.len == expected.len
	mut failed := []string{}
	for i, a in actual {
		if a != expected[i] {
			failed << '${a} expected ${expected[i]}: ${variants(bases[a.all_before(' ').int()])[a.all_after(' ').all_before(' ').int()]}'
		}
	}
	assert failed.len == 0, failed.len.str() + '\n' + failed#[..5].join('\n')
}
