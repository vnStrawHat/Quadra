//! Reading the YAML that manifests are written in into a [`Value`] (specs/13, "Data"): block
//! mappings and sequences by indentation, flow collections on one line, quoted and plain
//! scalars, block scalars, comments, and the first document. Anchors and tags are skipped.

mod flow;

use crate::value::{DEEPEST, ParseError, Value};
use flow::inline;

#[derive(Clone, Copy)]
struct Line<'a> {
    /// Its place in the text, from 1.
    number: usize,
    indent: usize,
    /// Without the indentation and the comment.
    text: &'a str,
    /// The whole line: a block scalar keeps it.
    raw: &'a str,
    blank: bool,
}

struct Reader<'a> {
    lines: Vec<Line<'a>>,
    at: usize,
}

/// Reads the first document of a YAML text.
pub fn parse_yaml(text: &str) -> Result<Value, ParseError> {
    let mut reader = Reader { lines: Vec::new(), at: 0 };
    for (number, raw) in text.lines().enumerate() {
        let kept = strip_comment(raw).trim_end_matches([' ', '\t']);
        let text = kept.trim_start_matches(' ');
        if text == "---" || text.starts_with("--- ") || text == "..." {
            if reader.lines.iter().any(|line| !line.blank) {
                break; // only the first document
            }
            continue;
        }
        let (number, indent) = (number + 1, kept.len() - text.len());
        reader.lines.push(Line { number, indent, text, raw, blank: text.is_empty() });
    }
    reader.block(0)
}

fn is_item(text: &str) -> bool {
    text == "-" || text.starts_with("- ")
}

impl<'a> Reader<'a> {
    fn wrong<T>(&self, message: &str) -> Result<T, ParseError> {
        let line = self.lines.get(self.at).or(self.lines.last()).map_or(1, |line| line.number);
        Err(ParseError { line, message: message.to_string() })
    }

    /// The next line that is not blank.
    fn peek(&mut self) -> Option<Line<'a>> {
        while self.lines.get(self.at).is_some_and(|line| line.blank) {
            self.at += 1;
        }
        self.lines.get(self.at).copied()
    }

    /// What starts at the next line: a sequence, a mapping, or a value by itself.
    fn block(&mut self, depth: usize) -> Result<Value, ParseError> {
        let Some(line) = self.peek() else { return Ok(Value::Null) };
        if depth > DEEPEST {
            return self.wrong("nested too deep");
        }
        if is_item(line.text) {
            return self.sequence(line.indent, depth);
        }
        if split_key(line.text).is_some() {
            return self.mapping(line.indent, depth);
        }
        self.at += 1;
        Ok(inline(line.text))
    }

    fn mapping(&mut self, indent: usize, depth: usize) -> Result<Value, ParseError> {
        let mut pairs = Vec::new();
        while let Some(line) = self.peek() {
            if line.indent != indent || is_item(line.text) {
                if line.indent > indent {
                    return self.wrong("unexpected indentation");
                }
                break;
            }
            let Some((key, rest)) = split_key(line.text) else { return self.wrong("a key was expected") };
            self.at += 1;
            pairs.push((key, self.value_after(indent, skip_props(rest), depth)?));
        }
        Ok(Value::Map(pairs))
    }

    /// What follows `key:` or `- `: a value on the line, a block scalar, or a block on the
    /// next lines.
    fn value_after(&mut self, indent: usize, rest: &str, depth: usize) -> Result<Value, ParseError> {
        if rest.starts_with(['|', '>']) {
            return Ok(self.block_scalar(indent, rest.starts_with('>')));
        }
        if !rest.is_empty() {
            return Ok(inline(rest));
        }
        match self.peek() {
            // A sequence may sit at the indentation of its key.
            Some(next) if next.indent > indent || (is_item(next.text) && next.indent == indent) => {
                self.block(depth + 1)
            }
            _ => Ok(Value::Null),
        }
    }

    fn sequence(&mut self, indent: usize, depth: usize) -> Result<Value, ParseError> {
        let mut items = Vec::new();
        while let Some(line) = self.peek().filter(|line| line.indent == indent && is_item(line.text)) {
            let after = line.text[1..].trim_start_matches(' ');
            let item = skip_props(after);
            if item.is_empty() {
                self.at += 1;
                items.push(self.value_after(indent, "", depth)?);
            } else if split_key(item).is_some() {
                // "- key: value" starts a mapping at the column of the key.
                let column = indent + line.text.len() - after.len();
                self.lines[self.at] = Line { indent: column, text: item, ..line };
                if depth >= DEEPEST {
                    return self.wrong("nested too deep");
                }
                items.push(self.mapping(column, depth + 1)?);
            } else {
                self.at += 1;
                items.push(inline(item));
            }
        }
        Ok(Value::List(items))
    }

    /// The lines further in than `indent`, as they are: joined by spaces when folded.
    fn block_scalar(&mut self, indent: usize, folded: bool) -> Value {
        let (mut out, mut content): (Vec<&str>, Option<usize>) = (Vec::new(), None);
        while let Some(line) = self.lines.get(self.at).filter(|line| line.blank || line.indent > indent) {
            if !line.blank {
                content.get_or_insert(line.indent);
            }
            let from = content.filter(|_| !line.blank);
            out.push(from.and_then(|from| line.raw.get(from..)).unwrap_or(""));
            self.at += 1;
        }
        while out.last() == Some(&"") {
            out.pop();
        }
        Value::Text(out.join(if folded { " " } else { "\n" }))
    }
}

/// Splits `key: rest`, the key in quotes or not. None: the text is not a key.
fn split_key(text: &str) -> Option<(String, &str)> {
    if text.is_empty() || text.starts_with(['{', '[']) || is_item(text) {
        return None;
    }
    if let Some(quote) = text.chars().next().filter(|c| matches!(c, '"' | '\'')) {
        let end = 1 + text[1..].find(quote)?;
        let rest = text[end + 1..].trim_start_matches(' ').strip_prefix(':')?;
        return Some((text[1..end].to_string(), rest.trim()));
    }
    let colon = text.find(": ").or_else(|| text.ends_with(':').then(|| text.len() - 1))?;
    (colon > 0).then(|| (text[..colon].trim().to_string(), text[colon + 1..].trim()))
}

/// Without an anchor (`&a`) or a tag (`!t`) before a value.
fn skip_props(mut text: &str) -> &str {
    while text.starts_with(['&', '!']) {
        let Some(space) = text.find(' ') else { return "" };
        text = text[space + 1..].trim_start_matches(' ');
    }
    text
}

/// Without a `#` comment, unless the `#` is in quotes.
fn strip_comment(line: &str) -> &str {
    let (bytes, mut quote) = (line.as_bytes(), 0u8);
    for (i, &c) in bytes.iter().enumerate() {
        if quote != 0 {
            quote = if c == quote { 0 } else { quote };
        } else if c == b'\'' || c == b'"' {
            quote = c;
        } else if c == b'#' && (i == 0 || matches!(bytes[i - 1], b' ' | b'\t')) {
            return &line[..i];
        }
    }
    line
}
