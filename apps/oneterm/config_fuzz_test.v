// Fuzz the config readers (specs/11, fuzz tests): terminal.json, ui_config.json and
// update_config.json are hand-edited files, so any text must load (leniently where the spec
// says so) or fall back to the defaults, never panic or hang.
module main

import json
import quadra.testing.fuzz

fn check_config_text(text string) {
	mut t := json.decode(TerminalConfig, strip_comments(text)) or { TerminalConfig{} }
	t.normalize('.tmp')
	mut u := json.decode(UiConfig, lenient_ui_document(text)) or { UiConfig{} }
	u.normalize()
	_ = u.lock_timeout_minutes()
	_ = json.decode(UpdateConfig, text) or { UpdateConfig{} }
}

fn test_fuzz_config_documents() {
	texts := [json.encode_pretty(TerminalConfig{}), json.encode_pretty(UiConfig{}),
		json.encode_pretty(UpdateConfig{}),
		'// comment\n{ /* block */ "font": {"size": 1e308, "family": "x"}, "scrollback": -1 }',
		'{"theme_name":"T","lock_after_idle_minutes":-5.5,"vertical_tabs":{"enabled":"yes","width":1e999,"collapsed":1}}',
		'{"lock_after_idle_minutes":"60","vertical_tabs":[true]} /* unclosed',
		'{"check_interval_hours": 4294967296, "channel": 7, "proxy_url": null}']
	for t in texts {
		check_config_text(t)
	}
	// Deep nesting fails as an error in both decoders (cJSON caps it; x.json2 by
	// tools/patches/v-json2-depth.diff).
	for open in ['[', '{"vertical_tabs":', '{"font": '] {
		check_config_text(open.repeat(100_000))
	}
	corpus := texts.map(it.bytes())
	n := fuzz.iters(10_000)
	mut r := fuzz.new_rng(0x636f6e66)
	mut w := fuzz.watch('config', n)
	for i in 0 .. n {
		src := r.mutate(corpus[r.intn(corpus.len)], corpus).bytestr()
		w.start()
		check_config_text(src)
		w.done(i, src.len, 2000)
	}
	w.finish(64)
}
