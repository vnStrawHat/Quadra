// CSS grid subset: track definitions and placement styles.
module layout

pub enum TrackUnit {
	px
	pct
	fr
	auto
	min_content
	max_content
}

pub struct TrackSize {
pub:
	unit  TrackUnit = .auto
	value f32
}

// Track is minmax(min, max). Plain sizes use the same value for both.
pub struct Track {
pub:
	min TrackSize
	max TrackSize
}

pub fn track_px(v f32) Track {
	return Track{TrackSize{.px, v}, TrackSize{.px, v}}
}

pub fn track_pct(v f32) Track {
	return Track{TrackSize{.pct, v}, TrackSize{.pct, v}}
}

pub fn fr(v f32) Track {
	return Track{TrackSize{.auto, 0}, TrackSize{.fr, v}}
}

pub fn track_auto() Track {
	return Track{}
}

pub fn min_content() Track {
	return Track{TrackSize{.min_content, 0}, TrackSize{.min_content, 0}}
}

pub fn max_content() Track {
	return Track{TrackSize{.max_content, 0}, TrackSize{.max_content, 0}}
}

pub fn minmax(lo TrackSize, hi TrackSize) Track {
	return Track{lo, hi}
}

pub fn size_px(v f32) TrackSize {
	return TrackSize{.px, v}
}

pub fn size_fr(v f32) TrackSize {
	return TrackSize{.fr, v}
}

pub fn repeat(n int, t Track) []Track {
	return []Track{len: n, init: t}
}

pub struct GridStyle {
pub mut:
	// container
	columns         []Track
	rows            []Track
	auto_rows       Track
	auto_cols       Track
	column_flow     bool // auto-placement fills columns first
	dense           bool
	justify_items   Align
	justify_content Justify
	align_content   Justify
	areas           []string // e.g. ['header header', 'nav main']
	fill_min        f32      // > 0: repeat(auto-fill, minmax(fill_min, 1fr)) columns
	// item
	col_start    int // 1-based line; 0 = auto
	col_span     int = 1
	row_start    int
	row_span     int = 1
	area         string
	justify_self AlignSelf
}

struct Area {
	row  int
	col  int
	rows int
	cols int
}

// parse_areas turns area rows into named rectangles (0-based).
fn parse_areas(rows []string) map[string]Area {
	mut out := map[string]Area{}
	for r, line in rows {
		for c, name in line.fields() {
			if name == '.' {
				continue
			}
			if a := out[name] {
				out[name] = Area{a.row, a.col, r - a.row + 1, if a.row == r {
					c - a.col + 1
				} else {
					a.cols
				}}
			} else {
				out[name] = Area{r, c, 1, 1}
			}
		}
	}
	return out
}
