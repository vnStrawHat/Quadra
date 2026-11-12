//! The geometry of charts (specs/13, "Charts"): a line becomes one vertical span a device
//! pixel column, and the numbers beside a chart are rounded and written short. Plain
//! functions.

/// A maximum rounded up to 1, 2, 2.5 or 5 times a power of ten: the top of an axis.
pub fn nice_max(v: f64) -> f64 {
    if v <= 0.0 {
        return 1.0;
    }
    let power = 10f64.powf(v.log10().floor());
    [1.0, 2.0, 2.5, 5.0].into_iter().map(|m| m * power).find(|top| v <= *top).unwrap_or(10.0 * power)
}

/// The largest of the values, and zero for none.
pub fn max_of<'a>(values: impl IntoIterator<Item = &'a f64>) -> f64 {
    values.into_iter().fold(0.0, |max, v| v.max(max))
}

/// The value at a share `t` of the way along the values, from 0 to 1: on the line between
/// the two samples around it.
pub fn value_at(values: &[f64], t: f64) -> f64 {
    let Some(last) = values.len().checked_sub(1) else { return 0.0 };
    let at = t * last as f64;
    let i = at.floor().max(0.0) as usize;
    if i >= last {
        return values[last];
    }
    values[i] + (values[i + 1] - values[i]) * (at - i as f64)
}

/// What a line covers of one pixel column, from the top of the plot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub top: f32,
    pub bottom: f32,
    /// The line's y at the middle of the column: where an area under it starts.
    pub y: f32,
    /// How tall the line is in the column; `top + thick + rise = bottom`.
    pub thick: f32,
}

/// The spans of values over `width` columns of a plot `height` tall whose top is `max`.
/// Each covers the line from the column's left edge to its right one: a line `thick` across
/// that falls d in a column is `thick * sqrt(1 + d²)` tall there. Nothing is rounded: a
/// span says where the line is to a fraction of a pixel, and what draws it must keep that.
// ponytail: a column reads the line at its two edges, so of more samples than columns the
// peaks between are not drawn; take the lowest and the highest sample of a column then.
pub fn line_spans(values: &[f64], width: usize, height: f32, max: f64, thick: f32) -> Vec<Span> {
    if values.is_empty() {
        return Vec::new();
    }
    let y_at = |column: usize| {
        let share = if max > 0.0 { value_at(values, column as f64 / width as f64) / max } else { 0.0 };
        height - share.clamp(0.0, 1.0) as f32 * height
    };
    let edges: Vec<f32> = (0..=width).map(y_at).collect();
    let span = |pair: &[f32]| {
        let (low, high) = (pair[0].min(pair[1]), pair[0].max(pair[1]));
        let tall = thick * (1.0 + (high - low) * (high - low)).sqrt();
        Span { top: low - tall / 2.0, bottom: high + tall / 2.0, y: (pair[0] + pair[1]) / 2.0, thick: tall }
    };
    edges.windows(2).map(span).collect()
}

/// Milliseconds since 1970 as the hour and the minute of their day, in UTC: "13:30".
pub fn hh_mm(ms: i64) -> String {
    let day = ms.rem_euclid(86_400_000);
    format!("{:02}:{:02}", day / 3_600_000, day / 60_000 % 60)
}

/// A number of an axis or a tooltip, short: 1.5k, 2M, 250, 0.25. It has three digits that
/// say something, or fewer.
pub fn short(v: f64) -> String {
    // Without the zeros after the point, and the point when nothing follows it.
    let cut = |text: String| match text.contains('.') {
        true => text.trim_end_matches('0').trim_end_matches('.').to_string(),
        false => text,
    };
    match v {
        v if v >= 1_000_000.0 => cut(format!("{:.1}", v / 1_000_000.0)) + "M",
        v if v >= 1000.0 => cut(format!("{:.1}", v / 1000.0)) + "k",
        v if v >= 100.0 => format!("{v:.0}"),
        v if v >= 10.0 => cut(format!("{v:.1}")),
        v => cut(format!("{v:.2}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_maximum_is_rounded_up_to_a_round_number() {
        let tops = [0.0, 7.0, 180.0, 230.0, 0.37, 1000.0].map(nice_max);
        assert_eq!(tops, [1.0, 10.0, 200.0, 250.0, 0.5, 1000.0]);
        assert_eq!((max_of(&[3.0, 9.0, 4.0]), max_of(&[])), (9.0, 0.0));
    }

    #[test]
    fn the_spans_of_a_line_cover_it_column_by_column() {
        // From 0 to 100 over 10 columns, 50 px tall under a top of 100: 5 px a column.
        let spans = line_spans(&[0.0, 100.0], 10, 50.0, 100.0, 2.0);
        assert_eq!((spans.len(), spans[0].y), (10, 47.5));
        for (x, span) in spans.iter().enumerate() {
            assert!(span.top <= 50.0 - (x + 1) as f32 * 5.0 + 1.0, "it reaches the next column's height");
            // 2 px across at a slope of 5 is 2 * sqrt(26) tall, and it falls 5 px.
            assert!(span.thick > 2.0 * 5.09 && span.thick < 2.0 * 5.1);
            assert!(span.bottom - span.top - span.thick > 4.99);
        }
        // A flat line is as tall as it is thick; values over the top stay in the plot.
        let flat = line_spans(&[50.0, 50.0, 50.0], 4, 50.0, 100.0, 1.0);
        assert!(flat.iter().all(|span| (span.top, span.bottom, span.thick) == (24.5, 25.5, 1.0)));
        assert_eq!(line_spans(&[500.0], 2, 50.0, 100.0, 1.0)[0].y, 0.0);
        assert!(line_spans(&[], 10, 50.0, 100.0, 1.0).is_empty());
    }

    #[test]
    fn values_between_samples_and_numbers_written_short() {
        assert_eq!((value_at(&[0.0, 10.0, 20.0], 0.25), value_at(&[0.0, 10.0, 20.0], 1.0)), (5.0, 20.0));
        assert_eq!((value_at(&[], 0.5), value_at(&[7.0], 0.5)), (0.0, 7.0));
        assert_eq!(
            [1500.0, 2000.0, 250.0, 0.25, 2_500_000.0].map(short),
            ["1.5k", "2k", "250", "0.25", "2.5M"]
        );
        assert_eq!([165.55, 12.34, 7.0, 0.5, 0.0].map(short), ["166", "12.3", "7", "0.5", "0"]);
        assert_eq!(hh_mm(1_793_966_400_000 + 90 * 60_000), "13:30");
    }
}
