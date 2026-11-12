//! Code that is not ours (specs/12, "Fuzzing"): whatever a line holds and whatever state it
//! starts in, every tokenizer gives tokens in order, inside the line and on the boundaries of
//! characters, in the time the line's length takes.

use std::time::{Duration, Instant};

use quadra_code::{Token, Tokenizer, tokenizer_for};
use quadra_testing::fuzz::{Rng, mutate, run};

/// A name for each tokenizer: the three with their own code, then every row of the table.
const LANGS: [&str; 30] = [
    "v",
    "yaml",
    "md",
    "c",
    "cpp",
    "cs",
    "java",
    "kotlin",
    "swift",
    "go",
    "rust",
    "js",
    "ts",
    "dart",
    "python",
    "ruby",
    "php",
    "lua",
    "sh",
    "pwsh",
    "bat",
    "sql",
    "make",
    "dockerfile",
    "json",
    "toml",
    "ini",
    "css",
    "html",
    "diff",
];

/// States a line can start in, and ones none gives: no state makes a tokenizer fail.
const STATES: [u32; 14] = [0, 0, 1, 2, 3, 7, 999, 1000, 1034, 1039, 1290, 1546, 2314, u32::MAX];

const CORPUS: [&str; 10] = [
    "pub fn (mut d Doc) move(m Move) int {\n\tx := 'a\\n${b.c}' // done\n\t/* a /* b */ c */ r'\\d' `a` @[heap]\n}\n",
    "base: &b !!map {a: *x, b: [1, 0x1F, ~]}\n- run: |\n    echo # not a comment\n\n  'it''s': \"q\\\"\" # c\n--- # doc\n",
    "# Title\n- [x] **done** and *maybe* `code` [a](http://b) $y$ <https://c>\n```rs\nfn f() {}\n```\n$$\nx^2\n$$\n",
    "#include <stdio.h>\nstatic int f(void) { return 0x1F; } // x\n/* a\nb */ char *s = \"a\\n\";\n",
    "fn f<'a>(x: &'a str) -> char { let r = r#\"q\"#; println!(\"{}\"); 'x' }\n#[derive(Debug)] let n = 1_000.5e-3;\n",
    "const a = `x ${b}\ny`; if (a) { return f(1) } // c\n@dataclass\ndef f(x: int) -> None:  # c\ns = f\"a\" + \"\"\"doc\n",
    "if [ -f \"$HOME/x\" ]; then echo 'a' # c\nGet-Item -Path $p -eq 1 <# a\nb #>\n:loop\nset X=%PATH% %~1\n",
    "<!DOCTYPE html>\n<a href=\"x\" data-id=1>T &amp; U</a> <!-- c\nd --> <img\n  src=\"y\" />\n.btn:hover, #id {\n  w: 10px;\n}\n",
    "[tool.x]\nname = \"q\" # c\nlocal x = [[a\nb]] --[[ c\nd ]] -- e\nSELECT COUNT(*) FROM t WHERE x = 'a' -- c\nall: $(OBJ)\n",
    "s := \"Xin chào ${thế_giới}\" // 漢字 😀\nkhóa: 'giá trị é' # ghi chú\n**đậm** `mã` [liên kết](http://ví.dụ) \\é\n",
];

/// What would be searched from every place to the end, were a search that failed not
/// remembered, and what opens without ever closing.
const UNITS: [&str; 20] = [
    "[", "<", "*a ", "_a ", "**a", "`", "$a ", "[a](", "[a]", "{a:,", "| ", "/*", "'", "\"${", "&a", "<!--",
    "#[", "- ", "%a", "é\\",
];

type Lang = (&'static str, Tokenizer);

fn check((lang, tokenizer): Lang, line: &str, state: u32, out: &mut Vec<Token>) -> u32 {
    out.clear();
    let next = tokenizer.tokenize(line, state, out);
    let mut from = 0;
    for token in out.iter() {
        let inside = from <= token.start && token.start < token.end && token.end <= line.len();
        assert!(inside, "{lang} gave {token:?} after {from} in {line:?} from the state {state}");
        let whole = line.is_char_boundary(token.start) && line.is_char_boundary(token.end);
        assert!(whole, "{lang} gave {token:?}, which cuts a character of {line:?}");
        from = token.end;
    }
    next
}

fn check_text(langs: &[Lang], text: &str) {
    let mut out = Vec::new();
    for lang in langs {
        let mut state = STATES[text.len() % STATES.len()];
        for line in text.split('\n') {
            state = check(*lang, line, state, &mut out);
        }
    }
}

fn langs() -> Vec<Lang> {
    let langs: Vec<_> = LANGS.iter().map(|name| (*name, tokenizer_for(name))).collect();
    assert!(langs.iter().all(|(_, tokenizer)| *tokenizer != Tokenizer::Plain));
    langs
}

#[test]
fn nothing_a_line_can_hold_makes_a_tokenizer_fail_or_cut_a_character() {
    let langs = langs();
    CORPUS.iter().for_each(|text| check_text(&langs, text));
    let corpus = CORPUS.map(str::as_bytes);
    let make = |rng: &mut Rng, _: usize| {
        if rng.chance(10) {
            let parts = 1 + rng.below(4);
            return (0..parts)
                .map(|_| rng.pick(&UNITS).repeat(1 + rng.below(300)))
                .collect::<String>()
                .into_bytes();
        }
        let from = *rng.pick(&corpus);
        mutate(rng, from, &corpus)
    };
    let check = |input: &[u8]| check_text(&langs, &String::from_utf8_lossy(input));
    run("tokenizers", 1500, Duration::from_secs(2), make, check);
}

#[test]
fn a_long_line_of_what_never_closes_is_read_in_the_time_its_length_takes() {
    let (langs, mut out) = (langs(), Vec::new());
    for unit in UNITS {
        let line = unit.repeat(40_000 / unit.len());
        for lang in &langs {
            let started = Instant::now();
            check(*lang, &line, 0, &mut out);
            let took = started.elapsed();
            assert!(took < Duration::from_millis(500), "{} took {took:?} for a line of {unit:?}", lang.0);
        }
    }
}
