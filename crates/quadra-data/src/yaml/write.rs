//! Writing a [`Value`] as block YAML (specs/13, "Data"): two spaces a level, a map in a
//! list on the dash's line, a text plain wherever YAML reads it back as the same text.

use crate::value::Value;

/// A value as block YAML, ending with a line break.
pub fn to_yaml(value: &Value) -> String {
    let mut out = String::new();
    match value {
        Value::Map(pairs) if !pairs.is_empty() => block(value, 0, &mut out),
        Value::List(items) if !items.is_empty() => block(value, 0, &mut out),
        _ => {
            out.push_str(&scalar(value));
            out.push('\n');
        }
    }
    out
}

/// What holds things, its lines `indent` spaces in.
fn block(value: &Value, indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    match value {
        Value::Map(pairs) => {
            for (key, inside) in pairs {
                out.push_str(&format!("{pad}{}:", text(key)));
                after(inside, indent, out);
            }
        }
        Value::List(items) => {
            for inside in items {
                match inside {
                    // What it holds, written 2 further in, its first line on the dash's.
                    Value::Map(pairs) if !pairs.is_empty() => dashed(inside, indent, out),
                    Value::List(items) if !items.is_empty() => dashed(inside, indent, out),
                    _ => out.push_str(&format!("{pad}- {}\n", scalar(inside))),
                }
            }
        }
        _ => {}
    }
}

fn dashed(value: &Value, indent: usize, out: &mut String) {
    let mut inner = String::new();
    block(value, indent + 2, &mut inner);
    out.push_str(&format!("{}- {}", " ".repeat(indent), &inner[indent + 2..]));
}

/// What follows a key's colon: a scalar on its line, or what it holds on the lines under it.
fn after(value: &Value, indent: usize, out: &mut String) {
    match value {
        Value::Map(pairs) if !pairs.is_empty() => {
            out.push('\n');
            block(value, indent + 2, out);
        }
        Value::List(items) if !items.is_empty() => {
            out.push('\n');
            block(value, indent + 2, out);
        }
        _ => out.push_str(&format!(" {}\n", scalar(value))),
    }
}

/// A value on one line: a scalar, or `{}` and `[]` for what holds nothing.
pub(crate) fn scalar(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(is) => is.to_string(),
        Value::Number(written) => written.clone(),
        Value::Text(words) => text(words),
        Value::List(_) => "[]".to_string(),
        Value::Map(_) => "{}".to_string(),
    }
}

/// A text plain where YAML reads it back as the same text, in double quotes otherwise.
fn text(words: &str) -> String {
    if plain(words) {
        return words.to_string();
    }
    let mut out = String::from("\"");
    for c in words.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn plain(words: &str) -> bool {
    const TAKEN: [&str; 12] =
        ["null", "~", "true", "false", "yes", "no", "on", "off", "y", "n", ".inf", ".nan"];
    let lower = words.to_ascii_lowercase();
    let number = words.parse::<f64>().is_ok()
        || lower.starts_with("0x")
        || lower.starts_with("0o")
        || lower.trim_start_matches(['-', '+']).starts_with(".inf");
    let lead = words.starts_with([
        '-', '?', ':', ',', '[', ']', '{', '}', '#', '&', '*', '!', '|', '>', '\'', '"', '%', '@', '`', ' ',
    ]);
    !words.is_empty()
        && !TAKEN.contains(&lower.as_str())
        && !number
        && !lead
        && !words.ends_with([' ', ':'])
        && !words.contains(": ")
        && !words.contains(" #")
        && !words.chars().any(char::is_control)
}
