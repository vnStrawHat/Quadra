//! URLs in plain text (specs/08, "URLs"; behaviour/04): the one detector, with one set of
//! rules, for Markdown, the editor and the terminal. Plain functions.

use std::ops::Range;

/// What a URL starts with, after a word boundary.
const PREFIXES: [&str; 3] = ["https://", "http://", "www."];

/// A URL found in a text: where it is, in bytes, and what it opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UrlSpan {
    pub range: Range<usize>,
    /// The text of the range; one that starts with `www.` gains `https://`.
    pub url: String,
}

/// Punctuation that ends the sentence around a URL, not the URL.
fn trails(c: char) -> bool {
    matches!(c, ')' | ']' | '}' | '>' | '.' | ',' | ';' | ':' | '!' | '?' | '\'' | '"' | '`')
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || !c.is_ascii()
}

/// The URLs of a line. A line with no `://` and no `www.` costs two searches and allocates
/// nothing.
pub fn find_urls(text: &str) -> Vec<UrlSpan> {
    let mut found = Vec::new();
    if !text.contains("://") && !text.contains("www.") {
        return found;
    }
    let (mut at, mut before) = (0, None::<char>);
    while let Some(c) = text[at..].chars().next() {
        let rest = &text[at..];
        let prefix = PREFIXES.iter().find(|prefix| rest.starts_with(**prefix));
        let Some(prefix) = prefix.filter(|_| !before.is_some_and(is_word)) else {
            (at, before) = (at + c.len_utf8(), Some(c));
            continue;
        };
        // It runs to a space, `<`, `>` or `"`, and loses the punctuation it ends with: but a
        // `)` whose `(` is in the URL stays.
        let stops = |c: char| c.is_whitespace() || matches!(c, '<' | '>' | '"');
        let (floor, run) = (at + prefix.len(), at + rest.find(stops).unwrap_or(rest.len()));
        let mut end = run;
        while let Some(last) = text[floor..end].chars().next_back().filter(|last| trails(*last)) {
            let inside = &text[floor..end - 1];
            if last == ')' && inside.matches('(').count() > inside.matches(')').count() {
                break;
            }
            end -= 1; // all of them are one byte
        }
        let url =
            if *prefix == "www." { format!("https://{}", &text[at..end]) } else { text[at..end].into() };
        if end > floor && url_openable(&url) {
            found.push(UrlSpan { range: at..end, url });
        }
        (at, before) = (run, text[..run].chars().next_back());
    }
    found
}

/// The gate for links in what a user did not write: `http`, `https` and `mailto` only, and
/// no control, direction or space characters. No other scheme (a file, a setting of the
/// system, a handler an app registered) is ever handed to the shell.
pub fn url_openable(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    let bad = |c: char| {
        let turns = matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{200e}' | '\u{200f}');
        c.is_control() || c.is_whitespace() || turns
    };
    ["https://", "http://", "mailto:"].iter().any(|scheme| lower.starts_with(scheme)) && !url.contains(bad)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn urls(text: &str) -> Vec<String> {
        find_urls(text).into_iter().map(|span| span.url).collect()
    }

    #[test]
    fn the_punctuation_after_a_url_is_not_of_it_but_a_bracket_it_opened_is() {
        assert_eq!(
            urls("see https://a.b/c. Then https://d.e/f, and \"https://g.h\"!"),
            ["https://a.b/c", "https://d.e/f", "https://g.h"]
        );
        assert_eq!(urls("<https://a.b/x> and `http://c.d/y`"), ["https://a.b/x", "http://c.d/y"]);
        assert!(urls("https://. https://").is_empty()); // a prefix alone is no URL
        assert_eq!(
            urls("(see https://en.wikipedia.org/wiki/Foo_(bar))"),
            ["https://en.wikipedia.org/wiki/Foo_(bar)"]
        );
        assert_eq!(urls("(https://a.b/c)."), ["https://a.b/c"]);
    }

    #[test]
    fn only_the_web_and_mail_are_opened() {
        assert!(url_openable("https://a.b") && url_openable("HTTP://a.b") && url_openable("mailto:x@y.z"));
        for bad in [
            "file:///C:/Windows/system32/calc.exe",
            "ms-settings:privacy",
            "javascript:alert(1)",
            "myapp://run",
            "ftp://a.b/x",
            "https://a.b/\u{202e}gnp",
            "https://a.b/x y",
            "https://a\u{7}",
        ] {
            assert!(!url_openable(bad), "{bad}");
        }
        // Nothing finds another scheme, or one glued to a word; a URL with a character
        // that turns the text round is not one.
        assert_eq!(urls("ftp://a.b/x xhttps://c.d www.e.org"), ["https://www.e.org"]);
        assert!(urls("https://a.b/\u{202e}gnp").is_empty());
    }

    #[test]
    fn a_range_is_in_bytes_whatever_the_letters_around() {
        let text = "Đọc https://d.org/đường/路 ngay";
        let found = find_urls(text);
        assert_eq!(found.len(), 1);
        assert_eq!(&text[found[0].range.clone()], "https://d.org/đường/路");
        assert!(find_urls("no links here, just a:b").is_empty());
    }
}
