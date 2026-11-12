//! Reading JSON into a [`Value`]: keys keep their order and numbers their spelling, which is
//! why this is not the reader the themes use.

use crate::value::{DEEPEST, ParseError, Value};

struct Reader<'a> {
    text: &'a str,
    at: usize,
}

/// Reads a JSON text whole: one value, and nothing but space after it.
pub fn parse_json(text: &str) -> Result<Value, ParseError> {
    let mut reader = Reader { text, at: 0 };
    let value = reader.value(0)?;
    reader.space();
    if reader.at < text.len() { reader.wrong("something follows the value") } else { Ok(value) }
}

impl Reader<'_> {
    fn wrong<T>(&self, message: &str) -> Result<T, ParseError> {
        let line = 1 + self.text[..self.at].matches('\n').count();
        Err(ParseError { line, message: message.to_string() })
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }

    fn space(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            self.at += 1;
        }
    }

    /// Takes a word if it is next.
    fn word(&mut self, word: &str) -> bool {
        let is = self.text[self.at..].starts_with(word);
        self.at += if is { word.len() } else { 0 };
        is
    }

    fn value(&mut self, depth: usize) -> Result<Value, ParseError> {
        self.space();
        match self.peek() {
            _ if depth > DEEPEST => self.wrong("nested too deep"),
            Some(b'{') => self.items(depth, b'}').map(Value::Map),
            Some(b'[') => {
                let items = self.items(depth, b']')?;
                Ok(Value::List(items.into_iter().map(|(_, value)| value).collect()))
            }
            Some(b'"') => self.string().map(Value::Text),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ if self.word("true") => Ok(Value::Bool(true)),
            _ if self.word("false") => Ok(Value::Bool(false)),
            _ if self.word("null") => Ok(Value::Null),
            Some(_) => self.wrong("a value was expected"),
            None => self.wrong("the text ends where a value was expected"),
        }
    }

    /// What is between brackets: values with commas between, each after a key and a colon
    /// when the bracket that closes is that of an object.
    fn items(&mut self, depth: usize, close: u8) -> Result<Vec<(String, Value)>, ParseError> {
        self.at += 1;
        let mut items = Vec::new();
        loop {
            self.space();
            if self.peek() == Some(close) && items.is_empty() {
                break;
            }
            let key = match close {
                b'}' if self.peek() != Some(b'"') => return self.wrong("a key in quotes was expected"),
                b'}' => {
                    let key = self.string()?;
                    self.space();
                    if !self.word(":") {
                        return self.wrong("a colon was expected after a key");
                    }
                    key
                }
                _ => String::new(),
            };
            items.push((key, self.value(depth + 1)?));
            self.space();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(c) if c == close => break,
                _ => return self.wrong("a comma or a closing bracket was expected"),
            }
        }
        self.at += 1;
        Ok(items)
    }

    fn number(&mut self) -> Result<Value, ParseError> {
        let from = self.at;
        while self.peek().is_some_and(|c| matches!(c, b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')) {
            self.at += 1;
        }
        let written = &self.text[from..self.at];
        match written.parse::<f64>() {
            Ok(_) => Ok(Value::Number(written.to_string())),
            Err(_) => self.wrong("not a number"),
        }
    }

    /// Four hex digits after `\u`.
    fn unit(&mut self) -> Result<u32, ParseError> {
        let digits = self.text.get(self.at..self.at + 4).and_then(|hex| u32::from_str_radix(hex, 16).ok());
        let Some(unit) = digits else { return self.wrong("four hex digits were expected after \\u") };
        self.at += 4;
        Ok(unit)
    }

    fn string(&mut self) -> Result<String, ParseError> {
        self.at += 1;
        let mut out = String::new();
        loop {
            // Up to the next quote or backslash: whole characters, copied as they are.
            let rest = &self.text[self.at..];
            let Some(stop) = rest.find(['"', '\\']) else { return self.wrong("a text is not closed") };
            out.push_str(&rest[..stop]);
            self.at += stop + 1;
            if rest.as_bytes()[stop] == b'"' {
                return Ok(out);
            }
            let escape = self.peek();
            self.at += 1;
            match escape {
                Some(b'n') => out.push('\n'),
                Some(b't') => out.push('\t'),
                Some(b'r') => out.push('\r'),
                Some(b'b') => out.push('\u{8}'),
                Some(b'f') => out.push('\u{c}'),
                Some(c @ (b'"' | b'\\' | b'/')) => out.push(c as char),
                Some(b'u') => {
                    let mut unit = self.unit()?;
                    // Two units for what is outside the first plane.
                    if (0xD800..0xDC00).contains(&unit) && self.word("\\u") {
                        unit =
                            0x10000 + ((unit - 0xD800) << 10) + (self.unit()?.wrapping_sub(0xDC00) & 0x3FF);
                    }
                    out.push(char::from_u32(unit).unwrap_or(char::REPLACEMENT_CHARACTER));
                }
                _ => {
                    self.at -= 1; // before what follows the backslash, whatever it is
                    return self.wrong("an escape that is none of JSON's");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_is_read_with_its_order_and_its_spelling() {
        let v = parse_json(r#"{"b": [1, 2.50, "x", null, true], "a": {"c": {}}, "t": "a\"\né😀"}"#);
        let v = v.unwrap();
        let Value::Map(pairs) = &v else { panic!("a map") };
        assert_eq!(pairs.iter().map(|(key, _)| key.as_str()).collect::<Vec<_>>(), ["b", "a", "t"]);
        let Some(Value::List(items)) = v.get("b") else { panic!("a list") };
        assert_eq!(items[1], Value::Number("2.50".to_string()));
        assert_eq!((&items[3], &items[4]), (&Value::Null, &Value::Bool(true)));
        assert_eq!(
            (v.get("b").unwrap().summary(), v.at(".a.c").unwrap().summary()),
            ("[5]".into(), "{0}".into())
        );
        assert_eq!(v.get("t"), Some(&Value::Text("a\"\né😀".to_string())));
    }

    #[test]
    fn what_is_not_json_is_an_error_with_its_line() {
        for (text, line) in
            [("{\"a\":", 1), ("{\n\"a\": 1,\n}", 3), ("[1 2]", 1), ("1 1", 1), ("\"a", 1), ("-", 1)]
        {
            assert_eq!(parse_json(text).unwrap_err().line, line, "{text}");
        }
        // Nested a thousand deep: an error, and no crash.
        assert_eq!(parse_json(&"[".repeat(1000)).unwrap_err().message, "nested too deep");
    }
}
