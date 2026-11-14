//! Where offsets are in a paragraph of several lines, and back: what a caret that moves up
//! and down, a click and a selection across lines need.

use std::ops::Range;

use quadra_prim::Rect;

use crate::paragraph::Paragraph;

impl Paragraph {
    /// The line a byte offset is on, and its x from the start of that line. An offset where
    /// a line was broken is at the start of the next line.
    pub fn place(&self, index: usize) -> (usize, f32) {
        let line = self.lines().partition_point(|line| line.start <= index).saturating_sub(1);
        let origin = self.lines().get(line).map_or(0.0, |line| line.origin);
        (line, self.x_at(index) - origin)
    }

    /// The caret stops of a line. Where a line was only broken, its end is the start of the
    /// next line: not a stop of this one.
    fn stops_of(&self, line: &crate::Line) -> &[(usize, f32)] {
        let broken = self.text().as_bytes().get(line.end).is_some_and(|&b| b != b'\n');
        let from = self.stops.partition_point(|&(at, _)| at < line.start);
        let to = self.stops.partition_point(|&(at, _)| at + broken as usize <= line.end);
        &self.stops[from..to]
    }

    /// The offset of the caret stop of a line nearest an x from the line's start. A line
    /// past the last is the end of the text.
    pub fn index_near(&self, line: usize, x: f32) -> usize {
        let Some(line) = self.lines().get(line) else { return self.text().len() };
        let far = |stop: &(usize, f32)| (stop.1 - line.origin - x).abs();
        let nearest = self.stops_of(line).iter().min_by(|a, b| far(a).total_cmp(&far(b)));
        nearest.map_or(line.start, |&(at, _)| at)
    }

    /// The offset of the last caret stop of a line: where End goes.
    pub fn line_end(&self, line: usize) -> usize {
        let Some(line) = self.lines().get(line) else { return self.text().len() };
        self.stops_of(line).last().map_or(line.start, |&(at, _)| at)
    }

    /// The boxes of a byte range, one for each line it is on, from the paragraph's corner. A
    /// range that goes on past the end of a line takes `past` more there: the line break
    /// that is in it shows.
    pub fn spans(&self, range: Range<usize>, past: f32) -> impl Iterator<Item = Rect> + '_ {
        let on = move |line: &&crate::Line| line.start <= range.end && line.end >= range.start;
        let (start, end) = (range.start, range.end);
        self.lines().iter().filter(on).filter_map(move |line| {
            let (from, to) = (start.max(line.start), end.min(line.end));
            let more = if end > line.end { past } else { 0.0 };
            let (x, width) = (self.x_at(from) - line.origin, self.x_at(to) - self.x_at(from) + more);
            (width > 0.0).then(|| Rect::new(x, line.y, width, line.height))
        })
    }
}

#[cfg(test)]
mod tests {
    use quadra_prim::Rect;

    use crate::{FakeBackend, Paragraph, TextStyle, TextSystem};

    /// "ab cd" and "efg" on two lines by a line break, the first broken again at 20 px: on
    /// the fake backend at size 10 a character is 6 px wide and a line 14 px high.
    #[test]
    fn offsets_are_found_on_their_lines_and_lines_give_offsets_back() {
        let mut ts = TextSystem::new(Box::new(FakeBackend::new()));
        let style = TextStyle { size: 10.0, ..TextStyle::default() };
        let mut p = Paragraph::new(&mut ts, "ab cd\nefg", style);
        p.wrap(&mut ts, 20.0);
        let lines: Vec<_> = p.lines().iter().map(|l| (l.start, l.end)).collect();
        assert_eq!(lines, [(0, 3), (3, 5), (6, 9)]);
        // Where the line was broken is the start of the next; before the line break is the
        // end of its line.
        assert_eq!(
            [p.place(1), p.place(3), p.place(5), p.place(6), p.place(9)],
            [(0, 6.0), (1, 0.0), (1, 12.0), (2, 0.0), (2, 18.0)]
        );
        assert_eq!([p.index_near(0, 7.0), p.index_near(1, 100.0), p.index_near(2, 8.0)], [1, 5, 7]);
        assert_eq!((p.index_near(2, -5.0), p.index_near(9, 0.0)), (6, 9));
        assert_eq!(p.index_near(0, 100.0), 2); // not 3: that is on the next line
        assert_eq!([p.line_end(0), p.line_end(1), p.line_end(2), p.line_end(3)], [2, 5, 9, 9]);
        // From "b" to "f": the rest of the first line, all of the second with its break, and
        // the start of the third.
        let spans: Vec<Rect> = p.spans(1..7, 3.0).collect();
        assert_eq!(
            spans,
            [
                Rect::new(6.0, 0.0, 12.0 + 3.0, 14.0),
                Rect::new(0.0, 14.0, 12.0 + 3.0, 14.0),
                Rect::new(0.0, 28.0, 6.0, 14.0)
            ]
        );
        assert_eq!(p.spans(4..4, 3.0).count(), 0); // nothing selected: no box
    }
}
