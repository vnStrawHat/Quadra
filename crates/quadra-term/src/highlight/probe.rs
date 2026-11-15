//! The structural matchers of output: addresses, dates and times, paths. Each claims a span
//! only where nothing before it did.

use super::Highlight as H;
use super::output::is_word;

/// Claims `start..end` for `class` when all of it is free.
fn claim(out: &mut [u8], start: usize, end: usize, class: H) {
    if out[start..end].iter().all(|&c| c == 0) {
        out[start..end].fill(class as u8);
    }
}

/// Runs `at` from each place a word may start, claiming what it finds whole.
fn each(chars: &[char], out: &mut [u8], class: H, at: impl Fn(&[char], usize) -> Option<usize>) {
    let mut i = 0;
    while i < chars.len() {
        let edge = !chars[i].is_whitespace() && (i == 0 || !is_word(chars[i - 1]));
        let found = edge.then(|| at(chars, i)).flatten();
        match found.filter(|&end| end > i && chars.get(end).is_none_or(|&c| !is_word(c))) {
            Some(end) => {
                claim(out, i, end, class);
                i = end;
            }
            None => i += 1,
        }
    }
}

fn hex_run(chars: &[char], i: usize, most: usize) -> usize {
    chars.get(i..).unwrap_or_default().iter().take(most).take_while(|c| c.is_ascii_hexdigit()).count()
}

/// `aa:bb:cc:dd:ee:ff`.
pub(super) fn mac(chars: &[char], out: &mut [u8]) {
    each(chars, out, H::Mac, |c, i| {
        let pair = |g: usize| hex_run(c, i + g * 3, 2) == 2 && (g == 5 || c.get(i + g * 3 + 2) == Some(&':'));
        (0..6).all(pair).then_some(i + 17)
    });
}

/// Groups of hex digits with one `::` among them, or eight groups.
pub(super) fn ipv6(chars: &[char], out: &mut [u8]) {
    each(chars, out, H::Ip, |c, i| {
        if i > 0 && c[i - 1] == ':' {
            return None;
        }
        let (mut k, mut groups, mut double) = (i, 0, false);
        if c.get(k..k + 2) == Some(&[':', ':'][..]) {
            (k, double) = (k + 2, true);
        }
        loop {
            let run = hex_run(c, k, 5);
            if run == 0 || run > 4 {
                break;
            }
            (k, groups) = (k + run, groups + 1);
            match c.get(k..k + 2) {
                Some([':', ':']) if !double => (k, double) = (k + 2, true),
                Some([':', h]) if h.is_ascii_hexdigit() => k += 1,
                _ => break,
            }
        }
        let dangling = c.get(k.wrapping_sub(1)) == Some(&':');
        ((1..=8).contains(&groups) && (double || (groups == 8 && !dangling))).then_some(k)
    });
}

/// `192.168.0.1`: four numbers up to 255, no digit or dot around.
pub(super) fn ipv4(chars: &[char], out: &mut [u8]) {
    let mut i = 0;
    while i < chars.len() {
        let free = out[i] == 0 && (i == 0 || !(chars[i - 1].is_ascii_digit() || chars[i - 1] == '.'));
        match free.then(|| quad(chars, i)).flatten() {
            Some(end) => {
                out[i..end].fill(H::Ip as u8);
                i = end;
            }
            None => i += 1,
        }
    }
}

fn quad(c: &[char], mut k: usize) -> Option<usize> {
    for part in 0..4 {
        let run = c[k..].iter().take(4).take_while(|d| d.is_ascii_digit()).count();
        let value: u32 = c[k..k + run].iter().fold(0, |v, d| v * 10 + d.to_digit(10).unwrap_or(0));
        if run == 0 || run > 3 || value > 255 {
            return None;
        }
        k += run;
        if part < 3 {
            (c.get(k) == Some(&'.')).then_some(())?;
            k += 1;
        }
    }
    c.get(k).is_none_or(|&d| !(d.is_ascii_digit() || d == '.')).then_some(k)
}

fn digits(c: &[char], k: usize, least: usize, most: usize) -> Option<usize> {
    let run = c[k.min(c.len())..].iter().take(most).take_while(|d| d.is_ascii_digit()).count();
    (run >= least).then_some(k + run)
}

/// After `w` (ASCII) at `k`.
fn word(c: &[char], k: usize, w: &str) -> Option<usize> {
    let mut at = k;
    for b in w.bytes() {
        (c.get(at) == Some(&char::from(b))).then_some(())?;
        at += 1;
    }
    Some(at)
}

fn spaces(c: &[char], k: usize, least: usize) -> Option<usize> {
    let run = c[k.min(c.len())..].iter().take_while(|d| d.is_whitespace()).count();
    (run >= least).then_some(k + run)
}

/// `14:30`, `09:30:45.123`, then a zone or AM/PM after spaces, if any.
fn time(c: &[char], k: usize) -> Option<usize> {
    let mut k = digits(c, word(c, digits(c, k, 1, 2)?, ":")?, 2, 2)?;
    if let Some(s) = word(c, k, ":").and_then(|k| digits(c, k, 2, 2)) {
        k = word(c, s, ".").and_then(|f| digits(c, f, 1, usize::MAX)).unwrap_or(s);
    }
    let zone = |z: usize| {
        let named = ["AM", "PM", "UTC", "GMT", "Z"].iter().find_map(|w| word(c, z, w));
        named.or_else(|| {
            let h = digits(c, word(c, z, "+").or_else(|| word(c, z, "-"))?, 2, 2)?;
            digits(c, word(c, h, ":").unwrap_or(h), 2, 2)
        })
    };
    Some(spaces(c, k, 0).and_then(zone).filter(|&e| c.get(e).is_none_or(|&x| !is_word(x))).unwrap_or(k))
}

const MONTHS: [&str; 12] =
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// A month's or a day's name and the small letters after it.
fn name(c: &[char], k: usize, names: &[&str]) -> Option<usize> {
    let end = names.iter().find_map(|n| word(c, k, n))?;
    Some(end + c[end..].iter().take_while(|x| x.is_ascii_lowercase()).count())
}

/// `2026-06-23`, `12/25/2023`, each with a time after a space or `T`; a time; the system log's
/// `Wed Oct 25 10:15:30 UTC 2023`; a month's or a day's name.
pub(super) fn dates(chars: &[char], out: &mut [u8]) {
    each(chars, out, H::DateTime, |c, i| {
        let sep = |k: usize| matches!(c.get(k), Some('-' | '/')).then_some(k + 1);
        let with_time = |end: usize| {
            let t = matches!(c.get(end), Some(' ' | 'T')).then(|| time(c, end + 1)).flatten();
            t.filter(|&e| c.get(e).is_none_or(|&x| !is_word(x))).unwrap_or(end)
        };
        let iso = || {
            let k = sep(digits(c, i, 4, 4)?)?;
            digits(c, sep(digits(c, k, 2, 2)?)?, 2, 2)
        };
        let us = || {
            let k = word(c, digits(c, word(c, digits(c, i, 2, 2)?, "/")?, 2, 2)?, "/")?;
            digits(c, k, 4, 4)
        };
        let log = || {
            let k = spaces(c, name(c, i, &DAYS)?, 1)?;
            let k = spaces(c, digits(c, spaces(c, name(c, k, &MONTHS)?, 1)?, 1, 2)?, 1)?;
            let k = time(c, k)?;
            let zone = spaces(c, k, 1).and_then(|s| ["UTC", "GMT"].iter().find_map(|w| word(c, s, w)));
            digits(c, spaces(c, zone.unwrap_or(k), 1)?, 4, 4)
        };
        iso()
            .map(with_time)
            .or_else(|| us().map(with_time))
            .or_else(|| time(c, i))
            .or_else(log)
            .or_else(|| name(c, i, &MONTHS))
            .or_else(|| name(c, i, &DAYS))
    });
}

fn segment(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '.' | '-' | '_')
}

/// Absolute paths (`/usr/bin`, `~/src`, `C:\Users`, `\\server`) not glued to a word before
/// them; relative ones with two separators or a file's extension (`src/main.rs`).
pub(super) fn paths(chars: &[char], out: &mut [u8]) {
    let (n, mut i) = (chars.len(), 0);
    let sep = |c: char| c == '/' || c == '\\';
    while i < n {
        let c = chars[i];
        let drive = c.is_ascii_alphabetic()
            && chars.get(i + 1) == Some(&':')
            && chars.get(i + 2).is_some_and(|&s| sep(s));
        if !(sep(c) || c == '~' || drive) || out[i] != 0 {
            i += 1;
            continue;
        }
        let mut start = i;
        if sep(c) {
            while start > 0 && segment(chars[start - 1]) && out[start - 1] == 0 {
                start -= 1;
            }
        }
        let mut end = i;
        while end < n && (segment(chars[end]) || matches!(chars[end], '/' | '\\' | '~' | ':')) {
            if chars[end] == ':' && end != start + 1 {
                break;
            }
            end += 1;
        }
        let span = &chars[start..end];
        let seps = span.iter().filter(|&&c| sep(c)).count();
        if seps > 0 && span.len() >= 2 {
            let drive = span.len() > 2 && span[0].is_ascii_alphabetic() && span[1] == ':';
            let absolute = sep(span[0]) || span[0] == '~' || drive;
            let last = &span[span.iter().rposition(|&c| sep(c)).map_or(0, |p| p + 1)..];
            let extension = last.iter().position(|&c| c == '.').is_some_and(|p| p > 0);
            let tag =
                if absolute { start == 0 || !is_word(chars[start - 1]) } else { seps >= 2 || extension };
            if tag {
                out[start..end].iter_mut().filter(|c| **c == 0).for_each(|c| *c = H::Path as u8);
            }
        }
        i = end.max(i + 1);
    }
}
