// Fuzz the theme loader (specs/11, fuzz tests): a theme file is user-supplied JSON, so any
// text must load or fail with an error, never panic or hang.
import quadra.kit
import quadra.testing.fuzz

fn test_fuzz_theme_json() {
	mut texts := [kit.dark().to_json(), kit.light().to_json(),
		'{"name": "x", "mode": "light", "palette": {"canvas": "#123456"}, "series": ["#fff"]}',
		'{"mode": "high_contrast", "terminal": {"ansi": ["#000000", "#ffffff"]}, "controls": {}}']
	for t in texts {
		kit.theme_from_json(t) or {}
	}
	// Deep nesting must fail as an error, not overflow x.json2's stack
	// (tools/patches/v-json2-depth.diff).
	for open in ['[', '{"a":', '{"mode": "dark", "palette": '] {
		kit.theme_from_json(open.repeat(100_000)) or { continue }
		assert false, 'nesting accepted: ${open}'
	}
	texts << '{"mode": "dark", "series": [' + '"#ffffff",'.repeat(5000) + '"#000"]}'
	corpus := texts.map(it.bytes())
	n := fuzz.iters(1500)
	mut r := fuzz.new_rng(0x7468656d)
	mut w := fuzz.watch('theme', n)
	for i in 0 .. n {
		src := r.mutate(corpus[r.intn(corpus.len)], corpus).bytestr()
		w.start()
		kit.theme_from_json(src) or {}
		w.done(i, src.len, 2000)
	}
	w.finish(64)
}
