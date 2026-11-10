// Fuzz the sessions document decoder (specs/11, fuzz tests): sessions.json is a file on disk
// anyone with access can edit, so any text must load or fail with an error, never panic.
module store

import quadra.testing.fuzz
import x.json2

fn full_document() string {
	mut s := session_named('prod')
	s.port = 2222
	s.username = 'ubuntu'
	s.auth_method = .private_key
	s.key_path = '/keys/test'
	s.jump_host = 7
	s.agent_forwarding = true
	s.forwards = [
		PortForward{
			kind:        .local
			bind_port:   8080
			target_host: 'localhost'
			target_port: 80
		},
		PortForward{
			kind:      .dynamic
			bind_port: 1080
		},
	]
	return encode_session_document(doc_of([entry_of(3, s), entry_of(9, session_named('b'))], 10))
}

fn test_fuzz_session_document() {
	texts := [full_document(), '[{ "label": "dev", "host": "localhost" }]',
		'{"schema_version":1,"sessions":[{"label":"alpha","host":"a","port":22,"group":null},{"label":"beta","host":"b","port":2200,"username":"deploy","auth_method":"private_key","key_path":"/keys/beta","color":"#112233","group":"infra"}]}',
		'{"schema_version":2,"next_session_id":3,"sessions":[{"id":5,"label":"keep","host":"a","group":null},{"label":"no-id","host":"b","group":null},{"id":5,"label":"dup","host":"c","group":null}]}']
	for t in texts {
		parse_session_document(t)!
	}
	// Regressions (tools/patches/v-json2-depth.diff): deep nesting overflowed x.json2's stack,
	// now an error past 256 levels; 255 still decode. An error at a newline or after tabs
	// sliced its context out of range in json2's checker_error (found by this test).
	for open in ['[', '{"a":', '{"sessions": '] {
		parse_session_document(open.repeat(100_000)) or { continue }
		assert false, 'nesting accepted: ${open}'
	}
	parse_json('['.repeat(255) + ']'.repeat(255))!
	parse_json('{"a": 1,\n' + '\t'.repeat(40) + ' x}') or {}
	parse_json('{"a":\n') or {}
	parse_json('{"a": 1}\n\n,') or {}
	// A value ending the text inside `[` or `{` looped forever, growing until memory ran out
	// (found by the long run: `[-2147483648`).
	for open in ['[1', '[-2147483648', '{"a":1', '{"a":[12', '[[0'] {
		parse_json(open) or { continue }
		assert false, 'unclosed accepted: ${open}'
	}
	assert parse_json('123')! == json2.Any(i64(123))
	corpus := texts.map(it.bytes())
	n := fuzz.iters(10_000)
	mut r := fuzz.new_rng(0x73657373)
	mut w := fuzz.watch('sessions', n)
	for i in 0 .. n {
		src := r.mutate(corpus[r.intn(corpus.len)], corpus).bytestr()
		w.start()
		if loaded := parse_session_document(src) {
			_ = encode_session_document(loaded.document)
		}
		w.done(i, src.len, 2000)
	}
	w.finish(64)
}
