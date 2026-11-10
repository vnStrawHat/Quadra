// Tokenizer benchmark (specs/18; run with -prod): a 2,000-line fence per language, the
// time and the heap bytes per line. The md preview tokenizes whole blocks at build time.
module main

import time
import quadra.kit.code

fn C.GC_get_total_bytes() usize

const samples = {
	'v':      ['pub fn handler(mut ctx Context, id int) !string { // entry',
		"\tname := 'item-\${id}' + ctx.prefix.to_upper()",
		'\tif id > 10 && ctx.flags & 0x1F != 0 { return error("bad") }', '}']
	'js':     ['export async function load(url, opts = {}) { // fetch it',
		'  const res = await fetch(`\${url}?q=\${opts.q}`, { method: "GET" });',
		'  if (res.status !== 200 && retries > 0x1f) { return null; }', '}']
	'rust':   [
		"pub fn parse<'a>(src: &'a str) -> Result<Vec<Token>, Error> { // entry",
		'    let raw = r#"a "b" c"#; println!("{}", src.len());',
		"    if src.is_empty() || c == 'x' { return Ok(Vec::new()); }",
		'}',
	]
	'python': ['@dataclass', 'def load(path: str, retries: int = 3) -> dict:  # read it',
		'    with open(path, "rb") as f:',
		"        return {'size': 0x1F, 'ok': True, 'name': f.name.upper()}"]
}

fn main() {
	for lang, rows in samples {
		mut lines := []string{cap: 2000}
		for i in 0 .. 2000 {
			lines << rows[i % rows.len]
		}
		tk := code.tokenizer_for('x.${lang}')
		mut toks := 0
		for _ in 0 .. 3 { // warm up
			mut st := 0
			for line in lines {
				t, next := tk.tokenize(line, st)
				st = next
				toks += t.len
			}
		}
		runs := 20
		b0 := C.GC_get_total_bytes()
		sw := time.new_stopwatch()
		for _ in 0 .. runs {
			mut st := 0
			for line in lines {
				_, next := tk.tokenize(line, st)
				st = next
			}
		}
		us := f64(sw.elapsed().microseconds()) / runs
		bytes := f64(C.GC_get_total_bytes() - b0) / runs / 2000
		println('${lang:-7} 2000 lines: ${us / 1000:.3} ms, ${us * 1000 / 2000:.0} ns/line, ${bytes:.0} B/line, ${toks / 3 / 2000} tokens/line')
	}
}
