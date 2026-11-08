// Chart geometry (specs/19, Charts): lines and areas become one vertical span per device
// pixel column, drawn as quads, so live charts never rasterize paths into the atlas.
module viz

import math
import quadra.prim
import quadra.kit

// nice_max rounds a maximum up to 1, 2, 2.5 or 5 times a power of ten (axis tops).
pub fn nice_max(v f64) f64 {
	if v <= 0 {
		return 1
	}
	p := math.pow(10, math.floor(math.log10(v)))
	for m in [1.0, 2.0, 2.5, 5.0, 10.0] {
		if v <= m * p {
			return m * p
		}
	}
	return 10 * p
}

pub fn max_of(series [][]f64) f64 {
	mut m := 0.0
	for s in series {
		for v in s {
			if v > m {
				m = v
			}
		}
	}
	return m
}

// value_at is the value at fractional position t in 0..1 (linear between samples).
pub fn value_at(values []f64, t f64) f64 {
	if values.len == 0 {
		return 0
	}
	if values.len == 1 {
		return values[0]
	}
	f := t * f64(values.len - 1)
	i := int(math.floor(f))
	if i >= values.len - 1 {
		return values.last()
	}
	return values[i] + (values[i + 1] - values[i]) * (f - f64(i))
}

// Span is the vertical extent of a line in one pixel column, from the top of the plot.
pub struct Span {
pub:
	top    f32
	bottom f32
	y      f32 // the line's y at the column's center (area tops)
	thick  f32 // the line's vertical extent: top + thick + rise = bottom
}

// line_spans maps values onto width columns of a plot height px tall with max at the top.
// Each column covers the line from its left to its right edge, thickened by its vertical
// extent: a line `thick` across with slope d is thick * sqrt(1 + d²) tall (the shader turns
// that into exact pixel coverage, gpu.LinePrim.column).
pub fn line_spans(values []f64, width int, height f32, max f64, thick f32) []Span {
	if width <= 0 || values.len == 0 {
		return []Span{}
	}
	ys := []f32{len: width + 1, init: y_of(value_at(values, f64(index) / f64(width)), height, max)}
	mut out := []Span{cap: width}
	for x in 0 .. width {
		a, b := ys[x], ys[x + 1]
		lo, hi := if a < b { a, b } else { b, a }
		d := hi - lo
		tv := thick * f32(math.sqrt(1 + d * d))
		out << Span{lo - tv / 2, hi + tv / 2, (a + b) / 2, tv}
	}
	return out
}

fn y_of(v f64, height f32, max f64) f32 {
	k := if max > 0 { v / max } else { 0 }
	return height - f32(math.min(math.max(k, 0), 1)) * height
}

// hh_mm formats unix ms as hh:mm (UTC).
pub fn hh_mm(ms i64) string {
	day := ms % 86_400_000
	return '${day / 3_600_000:02}:${day / 60_000 % 60:02}'
}

// short formats an axis or tooltip number: 1.5k, 250, 0.25.
pub fn short(v f64) string {
	if v >= 1_000_000 {
		return '${v / 1_000_000:.1f}M'.replace('.0M', 'M')
	}
	if v >= 1000 {
		return '${v / 1000:.1f}k'.replace('.0k', 'k')
	}
	if v == math.floor(v) {
		return '${int(v)}'
	}
	return '${v:.2f}'.trim_right('0')
}

// series_color is the theme's i-th chart color (the accent when a theme has none).
pub fn series_color(th &kit.Theme, i int) prim.Color {
	return if th.series.len > 0 { th.series[i % th.series.len] } else { th.p.accent }
}
