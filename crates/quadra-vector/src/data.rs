//! SVG path data: "M10 10 L20 20 a5 5 0 0 1 5 5 Z".

use quadra_prim::Point;

use crate::path::Path;

fn starts_number(b: u8) -> bool {
    b.is_ascii_digit() || b == b'-' || b == b'+' || b == b'.'
}

/// Where the number that starts at `from` ends: digits, one point, exponents.
fn number_end(text: &[u8], from: usize) -> usize {
    let (mut at, mut dot) = (from + 1, text[from] == b'.');
    while at < text.len() {
        match text[at] {
            b if b.is_ascii_digit() => at += 1,
            b'.' if !dot => (dot, at) = (true, at + 1),
            b'e' | b'E' if at + 1 < text.len() => {
                at += if matches!(text[at + 1], b'-' | b'+') { 2 } else { 1 };
            }
            _ => break,
        }
    }
    at.min(text.len())
}

fn number(text: &[u8]) -> f32 {
    std::str::from_utf8(text).ok().and_then(|text| text.parse().ok()).unwrap_or(0.0)
}

/// Every number in a text: "1,2 -3.5e1 .5-1" is 1, 2, -35, 0.5 and -1.
pub fn numbers(text: &str) -> Vec<f32> {
    let (text, mut out, mut at) = (text.as_bytes(), Vec::new(), 0);
    while at < text.len() {
        if starts_number(text[at]) {
            let end = number_end(text, at);
            out.push(number(&text[at..end]));
            at = end;
        } else {
            at += 1;
        }
    }
    out
}

/// Reads the number at `at` and moves `at` past it. A flag of an arc is one digit: they
/// may be written without anything between them ("a1 1 0 011 1").
fn read(data: &[u8], at: &mut usize, flag: bool) -> Option<f32> {
    while *at < data.len() && matches!(data[*at], b' ' | b',' | b'\n' | b'\t' | b'\r') {
        *at += 1;
    }
    let first = *data.get(*at)?;
    if first.is_ascii_alphabetic() && !matches!(first, b'e' | b'E') {
        return None;
    }
    let end = if flag { *at + 1 } else { number_end(data, *at) };
    let value = if flag { (first as f32) - (b'0' as f32) } else { number(&data[*at..end]) };
    *at = end;
    Some(value)
}

/// The control point a smooth curve continues with: the last one mirrored in the current
/// point, when the step before was a curve of its kind.
fn mirrored(before: u8, kinds: [u8; 2], ctrl: Point, at: Point) -> Point {
    match kinds.contains(&before.to_ascii_uppercase()) {
        true => Point::new(2.0 * at.x - ctrl.x, 2.0 * at.y - ctrl.y),
        false => at,
    }
}

impl Path {
    /// Appends what path data says. It stops at the first thing it cannot read: what came
    /// before stays.
    pub fn push_data(&mut self, data: &str) {
        let data = data.as_bytes();
        let (mut at, mut command, mut before) = (0, b' ', b' ');
        let mut ctrl = Point::ZERO;
        loop {
            while at < data.len() && !data[at].is_ascii_alphabetic() && !starts_number(data[at]) {
                at += 1;
            }
            if at >= data.len() {
                break;
            }
            if data[at].is_ascii_alphabetic() {
                command = data[at];
                at += 1;
                if command.eq_ignore_ascii_case(&b'z') {
                    self.close();
                    before = command;
                    continue;
                }
            }
            let kind = command.to_ascii_uppercase();
            let arity = match kind {
                b'M' | b'L' | b'T' => 2,
                b'H' | b'V' => 1,
                b'C' => 6,
                b'S' | b'Q' => 4,
                b'A' => 7,
                _ => break,
            };
            let mut n = [0.0f32; 7];
            let mut read_all = true;
            for (k, slot) in n.iter_mut().enumerate().take(arity) {
                match read(data, &mut at, kind == b'A' && (k == 3 || k == 4)) {
                    Some(value) => *slot = value,
                    None => read_all = false,
                }
            }
            if !read_all {
                break;
            }
            let now = self.current();
            let relative = command.is_ascii_lowercase();
            let (ox, oy) = if relative { (now.x, now.y) } else { (0.0, 0.0) };
            let point = |i: usize| Point::new(n[i] + ox, n[i + 1] + oy);
            match kind {
                b'M' => {
                    self.move_to(n[0] + ox, n[1] + oy);
                    command = if relative { b'l' } else { b'L' }; // more pairs are lines
                }
                b'L' => self.line_to(n[0] + ox, n[1] + oy),
                b'H' => self.line_to(n[0] + ox, now.y),
                b'V' => self.line_to(now.x, n[0] + oy),
                b'C' => {
                    self.cubic_to(point(0), point(2), point(4));
                    ctrl = point(2);
                }
                b'S' => {
                    self.cubic_to(mirrored(before, [b'C', b'S'], ctrl, now), point(0), point(2));
                    ctrl = point(0);
                }
                b'Q' => {
                    self.quad_to(n[0] + ox, n[1] + oy, n[2] + ox, n[3] + oy);
                    ctrl = point(0);
                }
                b'T' => {
                    ctrl = mirrored(before, [b'Q', b'T'], ctrl, now);
                    self.quad_to(ctrl.x, ctrl.y, n[0] + ox, n[1] + oy);
                }
                _ => self.arc_to(n[0], n[1], n[2], n[3] != 0.0, n[4] != 0.0, n[5] + ox, n[6] + oy),
            }
            before = command;
        }
    }
}
