//! A value (specs/13, "Data"): what JSON and YAML say, as a tree.

use std::fmt;

/// How deep a value may be nested in what is read: deeper is an error, not a crash.
pub(crate) const DEEPEST: usize = 128;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    /// As it was written: 1.50 stays 1.50.
    Number(String),
    Text(String),
    List(Vec<Value>),
    /// The keys in the order they were written.
    Map(Vec<(String, Value)>),
}

/// What is wrong in a text that was read, and on which line, from 1.
#[derive(Clone, Debug, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

/// How a key is written in a path: `.name`, or `["a key"]` when it is not a plain word.
pub(crate) fn step(key: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    if !key.is_empty() && key.chars().all(plain) { format!(".{key}") } else { format!("[{key:?}]") }
}

impl Value {
    /// The value under a key of a map.
    pub fn get(&self, key: &str) -> Option<&Value> {
        let Value::Map(pairs) = self else { return None };
        pairs.iter().find(|(name, _)| name == key).map(|(_, value)| value)
    }

    /// The value at a path such as `.spec.containers[0].image` or `.labels["a.b/c"]`.
    pub fn at(&self, path: &str) -> Option<&Value> {
        let (mut at, mut rest) = (self, path);
        while !rest.is_empty() {
            if let Some(after) = rest.strip_prefix("[\"") {
                // A key as `step` writes it: to the quote that closes it.
                let end = after.find("\"]")?;
                let key = after[..end].replace("\\\"", "\"").replace("\\\\", "\\");
                (at, rest) = (at.get(&key)?, &after[end + 2..]);
            } else if let Some(after) = rest.strip_prefix('[') {
                let end = after.find(']')?;
                let Value::List(items) = at else { return None };
                (at, rest) = (items.get(after[..end].parse::<usize>().ok()?)?, &after[end + 1..]);
            } else {
                let after = rest.strip_prefix('.')?;
                let end = after.find(['.', '[']).unwrap_or(after.len());
                (at, rest) = (at.get(&after[..end])?, &after[end..]);
            }
        }
        Some(at)
    }

    /// The value on one line: a scalar as it is written, `{n}` and `[n]` for what holds n.
    pub fn summary(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(is) => is.to_string(),
            Value::Number(written) => written.clone(),
            Value::Text(text) => format!("\"{text}\""),
            Value::List(items) => format!("[{}]", items.len()),
            Value::Map(pairs) => format!("{{{}}}", pairs.len()),
        }
    }

    /// The value with what is at `paths` left out: `.metadata.managedFields`, or the same
    /// without its first dot.
    pub fn without<S: AsRef<str>>(&self, paths: &[S]) -> Value {
        let dotted = |p: &str| if p.starts_with(['.', '[']) { p.to_string() } else { format!(".{p}") };
        let paths: Vec<String> = paths.iter().map(|p| dotted(p.as_ref())).collect();
        self.left(&paths, "")
    }

    fn left(&self, paths: &[String], path: &str) -> Value {
        let keep = |at: &String| !paths.contains(at);
        match self {
            Value::Map(pairs) => Value::Map(
                pairs
                    .iter()
                    .map(|(key, value)| (key, format!("{path}{}", step(key)), value))
                    .filter(|(_, at, _)| keep(at))
                    .map(|(key, at, value)| (key.clone(), value.left(paths, &at)))
                    .collect(),
            ),
            Value::List(items) => Value::List(
                items
                    .iter()
                    .enumerate()
                    .map(|(i, value)| (format!("{path}[{i}]"), value))
                    .filter(|(at, _)| keep(at))
                    .map(|(at, value)| value.left(paths, &at))
                    .collect(),
            ),
            scalar => scalar.clone(),
        }
    }

    /// The paths of what holds things, down to `levels` levels: what a view opens.
    pub fn holders(&self, levels: usize) -> std::collections::HashSet<String> {
        let mut all = std::collections::HashSet::new();
        let mut walk = vec![(self, String::new(), 0usize)];
        while let Some((value, path, depth)) = walk.pop() {
            for (_, step, inside) in value.inside() {
                let at = format!("{path}{step}");
                if depth < levels && !inside.inside().is_empty() {
                    all.insert(at.clone());
                    walk.push((inside, at, depth + 1));
                }
            }
        }
        all
    }

    /// What the value holds, each with how it is called there and the step to it in a path.
    pub(crate) fn inside(&self) -> Vec<(String, String, &Value)> {
        match self {
            Value::Map(pairs) => {
                pairs.iter().map(|(key, value)| (format!("{key}:"), step(key), value)).collect()
            }
            Value::List(items) => {
                let place = |(i, value)| (format!("[{i}]"), format!("[{i}]"), value);
                items.iter().enumerate().map(place).collect()
            }
            _ => Vec::new(),
        }
    }
}
