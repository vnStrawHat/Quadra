//! An output line's classes: the matchers in order, each on what the ones before left
//! (specs/18, "The view").

use super::Highlight as H;
use super::probe;

pub(super) fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub(super) fn scan(chars: &[char], out: &mut [u8]) {
    quotes(chars, out);
    permissions(chars, out);
    keywords(chars, out);
    probe::mac(chars, out);
    probe::ipv6(chars, out);
    probe::dates(chars, out);
    probe::ipv4(chars, out);
    probe::paths(chars, out);
    numbers(chars, out);
    options(chars, out);
    for (c, class) in chars.iter().zip(out.iter_mut()) {
        if *class == 0 {
            if matches!(
                c,
                '=' | ';' | '|' | '?' | '*' | '$' | '<' | '>' | '&' | '+' | '-' | ':' | '!' | '~' | '^'
            ) {
                *class = H::Operator as u8;
            } else if matches!(c, '(' | ')' | '[' | ']' | '{' | '}') {
                *class = H::Bracket as u8;
            }
        }
    }
}

/// `'..'` and `"..'`; one left open runs to the end.
fn quotes(chars: &[char], out: &mut [u8]) {
    let mut i = 0;
    while i < chars.len() {
        let q = chars[i];
        if q != '\'' && q != '"' {
            i += 1;
            continue;
        }
        let end = chars[i + 1..].iter().position(|&c| c == q).map_or(chars.len(), |p| i + p + 2);
        out[i..end].fill(H::Quoted as u8);
        i = end;
    }
}

/// The ten characters of `ls -l`'s mode at the start of a line, each by what it says.
fn permissions(chars: &[char], out: &mut [u8]) {
    let Some(mode) = chars.get(..10) else { return };
    let bits = mode[1..].iter().all(|c| matches!(c, 'r' | 'w' | 'x' | 's' | 't' | 'S' | 'T' | '-'));
    if !matches!(mode[0], 'b' | 'c' | 'd' | 'l' | 'p' | 's' | '-') || !bits {
        return;
    }
    out[0] = H::PermType as u8;
    for (c, class) in mode[1..].iter().zip(&mut out[1..10]) {
        *class = match c {
            'r' => H::PermRead,
            'w' => H::PermWrite,
            'x' => H::PermExec,
            '-' => H::PermNone,
            _ => H::PermSpecial,
        } as u8;
    }
}

/// The class of a keyword, lowercase; 0 for any other word.
fn keyword(word: &str) -> u8 {
    (match word {
        "debug" | "trace" | "verbose" => H::Debug,
        "closed" | "disconnected" | "exited" | "skipped" | "stopped" | "sudo" | "terminated" | "warning"
        | "warn" => H::Warn,
        "access" | "authentication" | "connection" | "disconnection" | "info" | "login" | "operation"
        | "password" | "permission" => H::Info,
        "can" | "correct" | "correctly" | "known" | "ok" | "pass" | "passed" | "success" | "successful"
        | "successfully" | "supported" | "true" | "valid" | "yes" => H::Success,
        "bad" | "cannot" | "denied" | "deprecated" | "disabled" | "error" | "errors" | "fail" | "failed"
        | "failure" | "false" | "important" | "incorrect" | "invalid" | "none" | "refused" | "unknown"
        | "unsupported" | "wrong" => H::Error,
        _ => return 0,
    }) as u8
}

/// Whole words of the lists, in any case; `not supported` is an error.
fn keywords(chars: &[char], out: &mut [u8]) {
    let (mut i, mut buf, mut last) = (0, String::new(), None::<(usize, bool)>);
    while i < chars.len() {
        if !is_word(chars[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && is_word(chars[i]) {
            i += 1;
        }
        let word = &chars[start..i];
        if word.len() > 14 || !word.iter().all(char::is_ascii) {
            last = None;
            continue;
        }
        buf.clear();
        buf.extend(word.iter().map(char::to_ascii_lowercase));
        let mut class = keyword(&buf);
        let mut from = start;
        // "not supported": the word before was `not`, one space before this one.
        if buf == "supported"
            && let Some((not_at, true)) = last
            && not_at + 4 == start
        {
            (class, from) = (H::Error as u8, not_at);
        }
        last = Some((start, buf == "not"));
        for c in &mut out[from..i] {
            if *c == 0 {
                *c = class;
            }
        }
    }
}

/// `--word`, `-x`, `-x=value` where no word goes on before them; `-1` is a number.
fn options(chars: &[char], out: &mut [u8]) {
    let (n, mut i) = (chars.len(), 0);
    while i < n {
        let lone = i == 0 || !is_word(chars[i - 1]);
        if out[i] != 0 || chars[i] != '-' || !lone || chars.get(i + 1).is_some_and(char::is_ascii_digit) {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        let long = chars.get(i) == Some(&'-');
        if long {
            i += 1;
            let had = chars.get(i).is_some_and(|&c| is_word(c));
            while i < n && (is_word(chars[i]) || matches!(chars[i], '-' | '=' | '.')) {
                i += 1;
            }
            if !had {
                continue;
            }
        } else if chars.get(i).is_some_and(|&c| is_word(c)) {
            while i < n && is_word(chars[i]) {
                i += 1;
            }
            if chars.get(i) == Some(&'=') {
                i += 1;
                while i < n && (is_word(chars[i]) || chars[i] == '.') {
                    i += 1;
                }
            }
        } else {
            continue;
        }
        for c in &mut out[start..i] {
            if *c == 0 {
                *c = H::Flag as u8;
            }
        }
    }
}

fn number_edge(c: char) -> bool {
    c.is_whitespace() || "()[]{},;|<>\"'+*\\=!?@#$%^&".contains(c)
}

/// `42`, `-7`, `1.5e3`, `0x1F`, `89%`, standing apart from words.
fn numbers(chars: &[char], out: &mut [u8]) {
    let (n, mut i) = (chars.len(), 0);
    let digit = |k: usize| chars.get(k).is_some_and(char::is_ascii_digit);
    let digits = |mut k: usize| {
        while digit(k) {
            k += 1;
        }
        k
    };
    while i < n {
        if out[i] != 0 {
            i += 1;
            continue;
        }
        let start = i;
        if chars[i] == '0' && matches!(chars.get(i + 1), Some('x' | 'X')) {
            i += 2;
            while i < n && chars[i].is_ascii_hexdigit() {
                i += 1;
            }
            if i > start + 2 && (i >= n || !is_word(chars[i])) {
                out[start..i].fill(H::Number as u8);
            }
            continue;
        }
        let negative = chars[i] == '-' && digit(i + 1) && (i == 0 || number_edge(chars[i - 1]));
        if !(digit(i) || (chars[i] == '.' && digit(i + 1)) || negative) {
            i += 1;
            continue;
        }
        i = digits(i + usize::from(negative));
        if chars.get(i) == Some(&'.') {
            i = digits(i + 1);
        }
        if matches!(chars.get(i), Some('e' | 'E')) {
            let sign = usize::from(matches!(chars.get(i + 1), Some('+' | '-')));
            let end = digits(i + 1 + sign);
            if end > i + 1 + sign {
                i = end;
            }
        }
        if chars.get(i) == Some(&'%') {
            i += 1;
        }
        let edges = (start == 0 || number_edge(chars[start - 1])) && (i >= n || number_edge(chars[i]));
        if edges && i > start {
            out[start..i].fill(H::Number as u8);
        }
    }
}
