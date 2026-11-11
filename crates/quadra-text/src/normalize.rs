//! Normalization for the Latin blocks (specs/08): composing a letter with the marks typed
//! after it, and taking the marks off for a search. Enough for Vietnamese and European text;
//! what is outside the tables passes through unchanged.

use std::borrow::Cow;

use crate::nfc_table::{COMPOSED, DECOMPOSED, MARKS};

fn unit(c: char) -> Option<u16> {
    u16::try_from(c as u32).ok()
}

/// The canonical combining class of a character: 0 for what is not a combining mark.
fn class(c: char) -> u8 {
    let (pairs, Some(key)) = (MARKS.as_chunks::<2>().0, unit(c)) else { return 0 };
    pairs.binary_search_by_key(&key, |pair| pair[0]).map_or(0, |at| pairs[at][1] as u8)
}

/// Whether a character is a combining mark the tables know.
pub fn is_mark(c: char) -> bool {
    class(c) > 0
}

/// The letter a base and a mark compose to, if there is one.
fn compose(base: char, mark: char) -> Option<char> {
    let (triples, key) = (COMPOSED.as_chunks::<3>().0, (unit(base)?, unit(mark)?));
    let at = triples.binary_search_by_key(&key, |t| (t[0], t[1])).ok()?;
    char::from_u32(triples[at][2] as u32)
}

/// A composed letter as the letter without its last mark, and that mark.
fn split(c: char) -> Option<(char, char)> {
    let (triples, key) = (DECOMPOSED.as_chunks::<3>().0, unit(c)?);
    let at = triples.binary_search_by_key(&key, |t| t[0]).ok()?;
    Some((char::from_u32(triples[at][1] as u32)?, char::from_u32(triples[at][2] as u32)?))
}

/// Appends a character taken apart all the way: its letter, then its marks.
fn apart(c: char, out: &mut Vec<char>) {
    match split(c) {
        Some((base, mark)) => {
            apart(base, out);
            out.push(mark);
        }
        None => out.push(c),
    }
}

/// The text with its combining marks composed onto their letters, whatever order they came
/// in: "e" with a circumflex and a dot below, or "ê" with a dot below, is one letter.
pub fn normalize_nfc(text: &str) -> Cow<'_, str> {
    if text.is_ascii() || !text.chars().any(is_mark) {
        return Cow::Borrowed(text);
    }
    let mut all = Vec::with_capacity(text.len());
    text.chars().for_each(|c| apart(c, &mut all));
    let (mut out, mut at) = (String::with_capacity(text.len()), 0);
    while at < all.len() {
        // A letter and the marks after it, in the order of their classes: marks of one class
        // keep the order they came in.
        let mut base = all[at];
        let marks = all[at + 1..].iter().take_while(|c| is_mark(**c)).count();
        let cluster = &mut all[at + 1..at + 1 + marks];
        cluster.sort_by_key(|mark| class(*mark));
        // A mark composes unless one of its own class stands uncomposed before it.
        let (mut rest, mut last) = (String::new(), 0);
        for &mark in cluster.iter() {
            match compose(base, mark).filter(|_| last < class(mark)) {
                Some(composed) => base = composed,
                None => {
                    rest.push(mark);
                    last = class(mark);
                }
            }
        }
        out.push(base);
        out.push_str(&rest);
        at += 1 + marks;
    }
    Cow::Owned(out)
}

/// The text without its accents, for a search typed without them: "Hải Phòng" is
/// "Hai Phong", and "đ" is "d".
pub fn fold_diacritics(text: &str) -> Cow<'_, str> {
    if text.is_ascii() {
        return Cow::Borrowed(text);
    }
    let bare = |mut c: char| {
        while let Some((base, _)) = split(c) {
            c = base;
        }
        match c {
            'đ' => 'd',
            'Đ' => 'D',
            c => c,
        }
    };
    Cow::Owned(text.chars().map(bare).filter(|c| !is_mark(*c)).collect())
}
