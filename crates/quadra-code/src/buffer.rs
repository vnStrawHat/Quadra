//! The text of a document (specs/13, "Code"): pieces of a few kilobytes in a row, and a
//! tree of the sums of their bytes and line breaks (`counts`). A change moves the bytes of
//! one piece and adds to a log of the sums; what asks where a line or an offset is finds
//! its piece down the tree and looks through that piece alone. Offsets are bytes, and are
//! between characters.

mod counts;

use std::borrow::Cow;
use std::ops::Range;

use counts::Counts;

/// A piece longer than this is cut into pieces of half of it at most.
const FULL: usize = 4096;
/// A piece shorter than this after something was deleted joins the next.
const SMALL: usize = FULL / 8;

struct Chunk {
    text: String,
    /// The line breaks in it.
    breaks: usize,
}

impl Chunk {
    fn new(text: &str) -> Chunk {
        Chunk { text: text.to_string(), breaks: breaks(text) }
    }
}

/// The tree of the sums of the pieces.
fn count(chunks: &[Chunk]) -> Counts {
    Counts::new(chunks.iter().map(|chunk| (chunk.text.len(), chunk.breaks)))
}

fn breaks(text: &str) -> usize {
    text.bytes().filter(|byte| *byte == b'\n').count()
}

/// Adds `text` to `out` in pieces: each ends after a line break where it holds one, so a
/// line is seldom in two pieces.
fn cut(mut text: &str, out: &mut Vec<Chunk>) {
    while text.len() > FULL / 2 {
        let most = text.floor_char_boundary(FULL / 2);
        let at = text[..most].rfind('\n').map_or(most, |at| at + 1);
        out.push(Chunk::new(&text[..at]));
        text = &text[at..];
    }
    out.push(Chunk::new(text));
}

pub struct Buffer {
    /// One at least; none is empty when there is another.
    chunks: Vec<Chunk>,
    /// The bytes and the line breaks of the pieces.
    counts: Counts,
    /// The file ended its lines with CR LF: the text here has LF alone, and
    /// [`Buffer::file_text`] writes them back.
    pub crlf: bool,
}

impl Buffer {
    pub fn new(text: &str) -> Buffer {
        let crlf = text.contains("\r\n");
        let mut chunks = Vec::new();
        cut(&if crlf { Cow::Owned(text.replace("\r\n", "\n")) } else { Cow::Borrowed(text) }, &mut chunks);
        let counts = count(&chunks);
        Buffer { chunks, counts, crlf }
    }

    pub fn len(&self) -> usize {
        self.counts.total().0
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// A text has a line more than it has line breaks: an empty one has one line.
    pub fn line_count(&self) -> usize {
        self.counts.total().1 + 1
    }

    /// The piece an offset is in, and where in it: at the start of a piece rather than at
    /// the end of the one before, but for the end of the text.
    fn locate(&self, offset: usize) -> (usize, usize) {
        assert!(offset <= self.len(), "offset {offset} is past the end, {}", self.len());
        let k = self.counts.within(offset, |sum| sum.0).min(self.chunks.len() - 1);
        (k, offset - self.counts.before(k).0)
    }

    pub fn line_start(&self, line: usize) -> usize {
        assert!(line < self.line_count(), "line {line} of {}", self.line_count());
        if line == 0 {
            return 0;
        }
        // The piece the line break before the line is in, and which of its breaks it is.
        let k = self.counts.within(line - 1, |sum| sum.1);
        let before = self.counts.before(k);
        let at = self.chunks[k].text.match_indices('\n').nth(line - before.1 - 1).map_or(0, |(at, _)| at + 1);
        before.0 + at
    }

    /// Where a line ends: before its line break, or at the end of the text.
    pub fn line_end(&self, line: usize) -> usize {
        if line + 1 < self.line_count() { self.line_start(line + 1) - 1 } else { self.len() }
    }

    /// The line an offset is in.
    pub fn line_of(&self, offset: usize) -> usize {
        let (k, at) = self.locate(offset);
        self.counts.before(k).1 + breaks(&self.chunks[k].text[..at])
    }

    /// The text between two offsets: lent when it is in one piece, as a line mostly is.
    pub fn slice(&self, range: Range<usize>) -> Cow<'_, str> {
        let (k, at) = self.locate(range.start);
        let length = range.end.saturating_sub(range.start);
        if at + length <= self.chunks[k].text.len() {
            return Cow::Borrowed(&self.chunks[k].text[at..at + length]);
        }
        assert!(range.end <= self.len(), "offset {} is past the end, {}", range.end, self.len());
        let mut out = String::with_capacity(length);
        out.push_str(&self.chunks[k].text[at..]);
        for chunk in &self.chunks[k + 1..] {
            out.push_str(&chunk.text[..chunk.text.len().min(length - out.len())]);
            if out.len() == length {
                break;
            }
        }
        Cow::Owned(out)
    }

    /// A line without its line break.
    pub fn line(&self, line: usize) -> Cow<'_, str> {
        self.slice(self.line_start(line)..self.line_end(line))
    }

    /// The bytes from an offset on.
    pub fn bytes(&self, from: usize) -> impl Iterator<Item = u8> + '_ {
        let (k, at) = self.locate(from);
        let first = self.chunks[k].text.as_bytes()[at..].iter().copied();
        first.chain(self.chunks[k + 1..].iter().flat_map(|chunk| chunk.text.bytes()))
    }

    /// The bytes before an offset, the nearest first.
    pub fn bytes_back(&self, before: usize) -> impl Iterator<Item = u8> + '_ {
        let (k, at) = self.locate(before);
        let first = self.chunks[k].text.as_bytes()[..at].iter().rev().copied();
        first.chain(self.chunks[..k].iter().rev().flat_map(|chunk| chunk.text.bytes().rev()))
    }

    /// Where `needle` is next, from an offset on.
    pub fn find(&self, needle: &str, from: usize) -> Option<usize> {
        let (k, at) = self.locate(from);
        // What is looked through: the end of what was, as long as a match can start in,
        // and a piece more each time.
        let (mut window, mut base) = (String::from(&self.chunks[k].text[at..]), from);
        for next in &self.chunks[k + 1..] {
            if let Some(found) = window.find(needle) {
                return Some(base + found);
            }
            let keep = window.ceil_char_boundary(window.len().saturating_sub(needle.len().saturating_sub(1)));
            base += keep;
            window.drain(..keep);
            window.push_str(&next.text);
        }
        window.find(needle).map(|found| base + found)
    }

    pub fn text(&self) -> String {
        self.chunks.iter().map(|chunk| chunk.text.as_str()).collect()
    }

    /// The text as its file has it: with CR LF where the file had them.
    pub fn file_text(&self) -> String {
        if self.crlf { self.text().replace('\n', "\r\n") } else { self.text() }
    }

    /// Puts `text` in place of the pieces `over`, cut again when it is too long for one.
    /// One piece for one adds to the sums; pieces that come or go count them again.
    fn replace(&mut self, over: Range<usize>, text: &str) {
        if over.len() == 1 && text.len() <= FULL && (!text.is_empty() || self.chunks.len() == 1) {
            let chunk = &mut self.chunks[over.start];
            let was = (chunk.text.len(), chunk.breaks);
            *chunk = Chunk::new(text);
            let change = (chunk.text.len() as isize - was.0 as isize, chunk.breaks as isize - was.1 as isize);
            self.counts.add(over.start, change);
            return;
        }
        let mut pieces = Vec::new();
        if text.len() > FULL {
            cut(text, &mut pieces);
        } else if !text.is_empty() || self.chunks.len() == over.len() {
            pieces.push(Chunk::new(text));
        }
        self.chunks.splice(over, pieces);
        self.counts = count(&self.chunks);
    }

    pub fn insert(&mut self, at: usize, text: &str) {
        let (k, local) = self.locate(at);
        let chunk = &mut self.chunks[k];
        chunk.text.insert_str(local, text);
        let added = breaks(text);
        chunk.breaks += added;
        if chunk.text.len() > FULL {
            let long = std::mem::take(&mut chunk.text);
            let mut pieces = Vec::new();
            cut(&long, &mut pieces);
            self.chunks.splice(k..k + 1, pieces);
            self.counts = count(&self.chunks);
        } else {
            self.counts.add(k, (text.len() as isize, added as isize));
        }
    }

    pub fn delete(&mut self, range: Range<usize>) {
        if range.end <= range.start {
            return;
        }
        let ((first, from), (last, to)) = (self.locate(range.start), self.locate(range.end));
        let mut left = String::from(&self.chunks[first].text[..from]);
        left.push_str(&self.chunks[last].text[to..]);
        // What is left joins the next piece when it is small, so that deleting does not
        // leave a row of crumbs.
        let mut end = last + 1;
        if left.len() < SMALL && end < self.chunks.len() {
            left.push_str(&self.chunks[end].text);
            end += 1;
        }
        self.replace(first..end, &left);
    }
}
