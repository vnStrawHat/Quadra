//! A logical line's classes, a character each: a prompt line (its sign, the path before it,
//! then the command), or output (`output.rs`); URLs over both.

use super::Highlight as H;
use super::{output, probe};

/// What the shell's marks made a line; `None`, its look decides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    Prompt,
    Output,
}

/// `out` gets a class a character of `chars` (`text` is the same line, for URLs); 0 is none.
/// `input_at` is where the shell marked the input of a prompt line.
pub(crate) fn scan(
    chars: &[char],
    text: &str,
    role: Option<Role>,
    input_at: Option<usize>,
    out: &mut Vec<u8>,
) {
    out.clear();
    out.resize(chars.len(), 0);
    match role {
        Some(Role::Prompt) => prompt_line(chars, out, marked_sign(chars, input_at)),
        Some(Role::Output) => output::scan(chars, out),
        None => match prompt_sign(chars) {
            Some(sign) => prompt_line(chars, out, Some(sign)),
            None => output::scan(chars, out),
        },
    }
    for url in quadra_text::find_urls(text) {
        let start = text[..url.range.start].chars().count();
        let len = text[url.range].chars().count();
        out[start..start + len].fill(H::Url as u8);
    }
}

pub(super) fn is_sign(c: char) -> bool {
    matches!(c, '$' | '#' | '%' | '>' | '<' | '❯' | '➜' | 'λ' | '→' | '»' | '\u{E0B0}'..='\u{E0B3}')
}

/// The sign of a line the shell marked a prompt: the last sign glyph before its input, else
/// the last character there that is not blank.
fn marked_sign(chars: &[char], input_at: Option<usize>) -> Option<usize> {
    let head = &chars[..input_at.unwrap_or(chars.len()).min(chars.len())];
    head.iter().rposition(|&c| is_sign(c)).or_else(|| head.iter().rposition(|c| !c.is_whitespace()))
}

/// The sign of a line that looks like a prompt, unmarked.
pub(super) fn prompt_sign(c: &[char]) -> Option<usize> {
    unix_sign(c).or_else(|| windows_sign(c))
}

/// `user@host:~$ `, `[user@host ~]# `, `(venv) ~/src% `, `$`: an optional `(venv) `, then a
/// `[..]` or a word with one of `@ : ~ / ]` (or nothing), then `$ # %` and a space or the end.
fn unix_sign(c: &[char]) -> Option<usize> {
    let venv = (c.first() == Some(&'('))
        .then(|| c.iter().position(|&x| x == ')'))
        .flatten()
        .filter(|&close| c[1..close].iter().all(|x| !x.is_whitespace()) && c.get(close + 1) == Some(&' '));
    venv.and_then(|close| unix_rest(c, close + 2)).or_else(|| unix_rest(c, 0))
}

fn unix_rest(c: &[char], i: usize) -> Option<usize> {
    let ends = |k: usize| {
        c.get(k).is_some_and(|&s| matches!(s, '$' | '#' | '%')) && c.get(k + 1).is_none_or(|&x| x == ' ')
    };
    if c.get(i) == Some(&'[')
        && let Some(close) = c[i + 1..].iter().position(|&x| x == ']')
        && ends(i + close + 2)
    {
        return Some(i + close + 2);
    }
    let word_end = c[i..].iter().position(|x| x.is_whitespace()).map_or(c.len(), |p| i + p);
    let special = |x: &char| matches!(x, '@' | ':' | '~' | '/' | ']');
    let longest = (i + 1..word_end).rev().find(|&k| ends(k) && c[i..k].iter().any(special));
    longest.or_else(|| ends(i).then_some(i))
}

/// `C:\path>`, `\\server\share>`, `PS C:\path>`, `PS>`: a root, then a path whose last
/// character is no space or `-`, then the first `>`.
fn windows_sign(c: &[char]) -> Option<usize> {
    if c.starts_with(&['P', 'S', '>']) {
        return Some(2);
    }
    let ps = c.starts_with(&['P', 'S', ' ']);
    let i = if ps { 3 } else { 0 };
    let rest = c.get(i..)?;
    let letters = rest.iter().take_while(|x| x.is_ascii_alphabetic()).count();
    let root = match rest {
        ['\\', '\\', ..] => 2,
        [..] if letters > 0 && rest.get(letters) == Some(&':') && (ps || letters == 1) => letters + 1,
        ['/', ..] if ps => 1,
        _ => return None,
    };
    let gt = i + root + rest[root..].iter().position(|&x| x == '>')?;
    let body = &c[i + root..gt];
    let bad = |x: &char| matches!(x, '<' | '|' | '"' | '*' | '?' | ':' | '\r' | '\n');
    let ok_end = body.last().is_some_and(|&x| !x.is_whitespace() && x != '-');
    (!body.iter().any(bad) && (ok_end || (ps && body.is_empty()))).then_some(gt)
}

/// Where a prompt's path starts when the prompt before its sign is a Windows path: after
/// `PS `, at a drive or `\\`.
fn windows_path(head: &[char]) -> Option<usize> {
    let start = if head.starts_with(&['P', 'S', ' ']) { 3 } else { 0 };
    let rest = head.get(start..)?;
    let drive = matches!(rest, [c, ':', sep, ..] if c.is_ascii_alphabetic() && matches!(sep, '\\' | '/'));
    (drive || rest.starts_with(&['\\', '\\'])).then_some(start)
}

/// The sign, the path before it, then the command after it.
fn prompt_line(chars: &[char], out: &mut [u8], sign: Option<usize>) {
    let n = chars.len();
    let first = || {
        let before_space = chars.iter().enumerate().take_while(|&(i, &c)| c != ' ' || i == 0);
        before_space.map(|(i, _)| i).find(|&i| is_sign(chars[i]))
    };
    let Some(sign) = sign.filter(|&i| i < n).or_else(first) else { return };
    out[sign] = H::PromptSign as u8;
    let (head, classes) = (&chars[..sign], &mut out[..sign]);
    match windows_path(head) {
        Some(start) => classes[start..].fill(H::Path as u8),
        None => probe::paths(head, classes),
    }
    let start = sign + 1 + chars[sign + 1..].iter().take_while(|&&c| c == ' ').count();
    if start < n {
        command(&chars[start..], &mut out[start..]);
    }
}

/// A command line: the first word is the command (again after `;`, `|`, `&&`, `||`), words
/// that start with `-` are options, the rest is left.
pub(super) fn command(chars: &[char], out: &mut [u8]) {
    let (n, mut i, mut expect) = (chars.len(), 0, true);
    while i < n {
        let c = chars[i];
        if c == ' ' {
            i += 1;
            continue;
        }
        let pair = |x: char| c == x && chars.get(i + 1) == Some(&x);
        if c == ';' || c == '|' || pair('&') {
            let len = if pair('|') || pair('&') { 2 } else { 1 };
            out[i..i + len].fill(H::Operator as u8);
            (i, expect) = (i + len, true);
            continue;
        }
        let start = i;
        while i < n && !matches!(chars[i], ' ' | ';' | '|') {
            i += 1;
        }
        if expect {
            out[start..i].fill(H::Command as u8);
            expect = false;
        } else if c == '-' {
            out[start..i].fill(H::Flag as u8);
        }
    }
}
