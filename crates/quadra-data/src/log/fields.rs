//! A JSON line as its message and fields (specs/13, "Data"): `request failed user=7
//! http.status=502`, its level and time left out.

use crate::json::parse_json;
use crate::value::Value;

const MESSAGE: [&str; 2] = ["msg", "message"];
const LEFT_OUT: [&str; 7] = ["level", "lvl", "severity", "time", "ts", "timestamp", "@timestamp"];

/// The message of a JSON object's line and `key=value` for its other fields; none for a line
/// that is not one.
pub fn as_fields(line: &str) -> Option<String> {
    if !line.trim_start().starts_with('{') {
        return None;
    }
    let Value::Map(pairs) = parse_json(line).ok()? else { return None };
    let message = pairs.iter().find(|(key, _)| MESSAGE.contains(&key.as_str()));
    let mut out = match message {
        Some((_, Value::Text(words))) => words.clone(),
        Some((_, value)) => plain(value),
        None => String::new(),
    };
    let rest =
        pairs.iter().filter(|(key, _)| !MESSAGE.contains(&key.as_str()) && !LEFT_OUT.contains(&key.as_str()));
    for (key, value) in rest {
        field(&mut out, key, value);
    }
    Some(out)
}

/// `key=value`, a map's fields as `key.inner=value`.
fn field(out: &mut String, key: &str, value: &Value) {
    if let Value::Map(pairs) = value {
        for (inner, value) in pairs {
            field(out, &format!("{key}.{inner}"), value);
        }
        return;
    }
    if !out.is_empty() {
        out.push(' ');
    }
    out.push_str(&format!("{key}={}", plain(value)));
}

/// A value as a field says it: as it is, or in double quotes with a space or a quote in it.
fn plain(value: &Value) -> String {
    match value {
        Value::Text(words) if words.is_empty() || words.contains([' ', '"', '=']) => format!("{words:?}"),
        Value::Text(words) => words.clone(),
        Value::List(items) => format!("[{}]", items.iter().map(plain).collect::<Vec<_>>().join(",")),
        Value::Map(pairs) => format!("{{{}}}", pairs.len()),
        other => other.summary(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_json_line_is_its_message_and_fields_without_level_and_time() {
        let line = r#"{"time":"12:00","level":"info","msg":"Error budget fine","user":7,"http":{"status":502},"tags":["a","b"],"why":"no route"}"#;
        assert_eq!(
            as_fields(line).as_deref(),
            Some(r#"Error budget fine user=7 http.status=502 tags=[a,b] why="no route""#)
        );
        assert_eq!(as_fields(r#"{"level":"warn","ok":true}"#).as_deref(), Some("ok=true"));
        assert_eq!((as_fields("INFO plain"), as_fields("{broken")), (None, None));
    }
}
