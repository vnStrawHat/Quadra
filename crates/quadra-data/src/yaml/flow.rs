//! YAML values on one line: flow collections (`{a: 1, b: [x, y]}`) and scalars.

use crate::value::{DEEPEST, Value};

struct Flow<'a> {
    text: &'a str,
    at: usize,
}

/// The value written after `key:` or `- `.
pub(super) fn inline(text: &str) -> Value {
    let text = text.trim();
    if text.starts_with(['{', '[']) { Flow { text, at: 0 }.value(0) } else { plain_scalar(text) }
}

impl Flow<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }

    fn space(&mut self) {
        while self.peek() == Some(b' ') {
            self.at += 1;
        }
    }

    /// Takes a character if it is next.
    fn take(&mut self, c: u8) -> bool {
        let is = self.peek() == Some(c);
        self.at += usize::from(is);
        is
    }

    fn value(&mut self, depth: usize) -> Value {
        self.space();
        let open = self.peek();
        if !matches!(open, Some(b'{' | b'[')) || depth > DEEPEST {
            return plain_scalar(&self.word(false));
        }
        self.at += 1;
        let (mut keys, mut items) = (Vec::new(), Vec::new());
        loop {
            self.space();
            if self.peek().is_none() || self.take(b'}') || self.take(b']') {
                break;
            }
            let before = self.at;
            if open == Some(b'{') {
                keys.push(self.word(true));
                self.space();
                self.take(b':');
            }
            items.push(self.value(depth + 1));
            self.space();
            self.take(b',');
            if self.at == before {
                // Nothing was read (a colon by itself): past it, or this never ends.
                self.at += self.text[self.at..].chars().next().map_or(1, char::len_utf8);
            }
        }
        match open {
            Some(b'{') => Value::Map(keys.into_iter().zip(items).collect()),
            _ => Value::List(items),
        }
    }

    /// A scalar, in quotes or plain, up to what ends it in a flow collection (and a colon,
    /// for a key).
    fn word(&mut self, key: bool) -> String {
        let (start, bytes) = (self.at, self.text.as_bytes());
        if let Some(quote) = self.peek().filter(|c| matches!(c, b'"' | b'\'')) {
            self.at += 1;
            while self.peek().is_some_and(|c| c != quote) {
                // What follows a backslash in double quotes is not the end, whatever it is.
                let escaped = quote == b'"' && self.peek() == Some(b'\\');
                self.at += usize::from(escaped);
                self.at +=
                    self.text.get(self.at..).and_then(|rest| rest.chars().next()).map_or(1, char::len_utf8);
            }
            self.at = (self.at + 1).min(bytes.len());
            let word = &self.text[start..self.at];
            return if key { unquote(word) } else { word.to_string() };
        }
        while let Some(c) = self.peek() {
            let ends = matches!(c, b',' | b'}' | b']')
                || (c == b':' && (key || bytes.get(self.at + 1) == Some(&b' ')));
            if ends {
                break;
            }
            self.at += 1;
        }
        self.text[start..self.at].trim().to_string()
    }
}

fn unquote(word: &str) -> String {
    let inside = |quote: char| word.strip_prefix(quote).and_then(|rest| rest.strip_suffix(quote));
    match (word.len() >= 2, inside('"'), inside('\'')) {
        (true, Some(text), _) => text.replace("\\n", "\n").replace("\\\"", "\"").replace("\\\\", "\\"),
        (true, _, Some(text)) => text.replace("''", "'"),
        _ => word.to_string(),
    }
}

pub(super) fn plain_scalar(text: &str) -> Value {
    if text.len() >= 2 && text.starts_with(['"', '\'']) {
        return Value::Text(unquote(text));
    }
    match text {
        "" | "~" | "null" | "Null" | "NULL" => Value::Null,
        "true" | "True" | "TRUE" => Value::Bool(true),
        "false" | "False" | "FALSE" => Value::Bool(false),
        _ if is_number(text) => Value::Number(text.to_string()),
        _ => Value::Text(text.to_string()),
    }
}

fn is_number(word: &str) -> bool {
    let bytes = word.as_bytes();
    if bytes.len() > 2 && bytes[0] == b'0' && matches!(bytes[1], b'x' | b'o') {
        return bytes[2..].iter().all(u8::is_ascii_hexdigit);
    }
    let count = |of: fn(&u8) -> bool| bytes.iter().filter(|c| of(c)).count();
    let signs_in_place = bytes.iter().enumerate().all(|(i, c)| match c {
        b'0'..=b'9' | b'.' | b'_' | b'e' | b'E' => true,
        b'-' | b'+' => i == 0 || matches!(bytes[i - 1], b'e' | b'E'),
        _ => false,
    });
    let (dots, exponents) = (count(|c| *c == b'.'), count(|c| matches!(c, b'e' | b'E')));
    signs_in_place
        && count(u8::is_ascii_digit) > 0
        && dots <= 1
        && exponents <= 1
        && matches!(bytes[0], b'0'..=b'9' | b'-' | b'+' | b'.')
}
