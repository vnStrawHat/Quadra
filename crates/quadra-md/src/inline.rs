//! The inlines of a block's text: `**bold**`, `*italic*` (or `_italic_` between words),
//! `~~struck~~`, `` `code` ``, `$math$`, `[text](url)`, `![alt](src)`, `<url>`, bare URLs
//! (the detector of `quadra-text`) and backslash escapes. Comments show nothing.

use quadra_text::find_urls;

/// A run of a block's text in one style.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Span {
    /// What it says: the source of a formula for math, the words in place of an image.
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    pub math: bool,
    /// Where it leads, when it is a link.
    pub link: Option<String>,
    /// The source of an image.
    pub image: Option<String>,
}

/// How deep links in the labels of links are read; deeper, a label is plain text.
const DEEPEST: usize = 16;

pub fn parse_inline(source: &str) -> Vec<Span> {
    inline(source, 0)
}

/// Letters, digits and what is not ASCII: what `_` does not open or close between.
fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c >= 0x80
}

/// The `$` that closes the math opened at `i`. The one that opens is followed by something
/// other than a space; the one that closes comes after something other than a space, and no
/// digit follows it ("$5 and $10" stays text).
fn math_end(b: &[u8], i: usize) -> Option<usize> {
    if matches!(b.get(i + 1), None | Some(b' ' | b'$')) {
        return None;
    }
    let mut j = i + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 1,
            b'$' => return (b[j - 1] != b' ' && !b.get(j + 1).is_some_and(u8::is_ascii_digit)).then_some(j),
            _ => {}
        }
        j += 1;
    }
    None
}

/// Where the bracket that each `[` opens is closed, in the order of the `[`.
fn brackets(b: &[u8]) -> Vec<(usize, usize)> {
    let (mut open, mut pairs) = (Vec::new(), Vec::new());
    for (at, c) in b.iter().enumerate() {
        match c {
            b'[' => open.push(at),
            b']' => pairs.extend(open.pop().map(|from| (from, at))),
            _ => {}
        }
    }
    pairs.sort_unstable();
    pairs
}

/// The text being read, with what was looked for and is known not to be there further on:
/// a search that failed is not made again, so a text costs what its length costs.
struct Reader<'a> {
    s: &'a str,
    pairs: Vec<(usize, usize)>,
    /// From where on there is no `)`.
    no_paren: usize,
    no_tick: bool,
    no_angle: bool,
    no_comment_end: bool,
}

impl<'a> Reader<'a> {
    /// `pattern` after `from`, unless it was looked for before and not found.
    fn find(&self, from: usize, pattern: &str, known_gone: bool) -> Option<usize> {
        if known_gone { None } else { self.s[from..].find(pattern).map(|at| from + at) }
    }

    /// `[label](target)` with its `[` at `i`: the label, the target without a title, and
    /// where it ends.
    fn link(&mut self, i: usize) -> Option<(&'a str, &'a str, usize)> {
        let (s, at) = (self.s, self.pairs.binary_search_by_key(&i, |pair| pair.0).ok()?);
        let close = self.pairs[at].1;
        if s.as_bytes().get(close + 1) != Some(&b'(') {
            return None;
        }
        let from = close + 2;
        let within = self.no_paren.clamp(from, s.len());
        let Some(end) = s[from..within].find(')').map(|at| from + at) else {
            self.no_paren = self.no_paren.min(from);
            return None;
        };
        let target = s[from..end].trim();
        let target = target.split(' ').next().unwrap_or(target); // `(url "title")`
        Some((&s[i + 1..close], target.trim_matches(['<', '>']), end + 1))
    }
}

fn inline(s: &str, depth: usize) -> Vec<Span> {
    let b = s.as_bytes();
    let (mut out, mut text) = (Vec::new(), String::new());
    let (mut bold, mut italic, mut strike) = (false, false, false);
    let pairs = if s.contains('[') { brackets(b) } else { Vec::new() };
    let mut r =
        Reader { s, pairs, no_paren: usize::MAX, no_tick: false, no_angle: false, no_comment_end: false };
    let (urls, mut url) = (find_urls(s), 0);
    let mut i = 0;
    // What was read so far goes out as a span in the style of the moment.
    macro_rules! emit {
        () => {
            if !text.is_empty() {
                out.push(Span { text: std::mem::take(&mut text), bold, italic, strike, ..Span::default() });
            }
        };
    }
    macro_rules! styled {
        ($text:expr) => {
            Span { text: $text.to_string(), bold, italic, strike, ..Span::default() }
        };
    }
    while i < b.len() {
        while url < urls.len() && urls[url].range.start < i {
            url += 1;
        }
        if let Some(found) = urls.get(url).filter(|found| found.range.start == i) {
            emit!();
            out.push(Span { link: Some(found.url.clone()), ..styled!(&s[found.range.clone()]) });
            i = found.range.end;
            continue;
        }
        let (c, next) = (b[i], b.get(i + 1).copied());
        match c {
            b'\\' if next.is_some_and(|next| !is_word(next) && next != b'\n') => {
                text.push(next.unwrap_or(b'\\') as char);
                i += 2;
                continue;
            }
            b'`' => {
                if let Some(end) = r.find(i + 1, "`", r.no_tick) {
                    emit!();
                    out.push(Span { code: true, ..styled!(&s[i + 1..end]) });
                    i = end + 1;
                    continue;
                }
                r.no_tick = true;
            }
            b'$' => {
                if let Some(end) = math_end(b, i) {
                    emit!();
                    out.push(Span { math: true, ..styled!(&s[i + 1..end]) });
                    i = end + 1;
                    continue;
                }
            }
            b'~' if next == Some(b'~') => {
                emit!();
                (strike, i) = (!strike, i + 2);
                continue;
            }
            b'*' | b'_' if next == Some(c) => {
                emit!();
                (bold, i) = (!bold, i + 2);
                continue;
            }
            b'*' | b'_' if c == b'*' || i == 0 || !is_word(b[i - 1]) || italic => {
                let closes = italic && (c == b'*' || !next.is_some_and(is_word));
                let opens = !italic && next.is_some_and(|next| next != b' ');
                if closes || opens {
                    emit!();
                    (italic, i) = (!italic, i + 1);
                    continue;
                }
            }
            b'!' if next == Some(b'[') => {
                if let Some((alt, source, end)) = r.link(i + 1) {
                    emit!();
                    out.push(Span { text: alt.into(), image: Some(source.into()), ..Span::default() });
                    i = end;
                    continue;
                }
            }
            b'[' if depth < DEEPEST => {
                if let Some((label, target, end)) = r.link(i) {
                    emit!();
                    out.extend(inline(label, depth + 1).into_iter().map(|inner| Span {
                        bold: inner.bold || bold,
                        italic: inner.italic || italic,
                        strike: inner.strike || strike,
                        link: Some(target.to_string()),
                        ..inner
                    }));
                    i = end;
                    continue;
                }
            }
            b'<' if s[i..].starts_with("<!--") => {
                if let Some(end) = r.find(i + 4, "-->", r.no_comment_end) {
                    i = end + 3; // a comment shows nothing
                    continue;
                }
                r.no_comment_end = true;
            }
            b'<' if s[i + 1..].starts_with("http://") || s[i + 1..].starts_with("https://") => {
                if let Some(end) = r.find(i + 1, ">", r.no_angle) {
                    emit!();
                    out.push(Span { link: Some(s[i + 1..end].to_string()), ..styled!(&s[i + 1..end]) });
                    i = end + 1;
                    continue;
                }
                r.no_angle = true;
            }
            _ => {}
        }
        // Nothing of the above: the character is text.
        let end = (i + 1..=b.len()).find(|&end| s.is_char_boundary(end)).unwrap_or(b.len());
        text.push_str(&s[i..end]);
        i = end;
    }
    emit!();
    out
}
