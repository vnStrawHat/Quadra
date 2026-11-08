// Layout style: the flexbox (and later grid) subset described in specs/05-layout.md.
module layout

import quadra.prim

pub enum Unit {
	auto
	px
	pct
	fill
}

pub struct Length {
pub:
	unit  Unit
	value f32
}

pub const auto = Length{}

pub fn px(v f32) Length {
	return Length{.px, v}
}

pub fn pct(v f32) Length {
	return Length{.pct, v}
}

pub fn fill() Length {
	return Length{
		unit: .fill
	}
}

pub fn (l Length) is_auto() bool {
	return l.unit == .auto
}

// resolve returns the length in px against a parent size, or `unknown` when it
// depends on content (auto) or on an unknown parent size.
pub fn (l Length) resolve(parent f32) f32 {
	return match l.unit {
		.px {
			l.value
		}
		.pct {
			if parent >= 0 && parent < unbounded {
				l.value / 100 * parent
			} else {
				unknown
			}
		}
		.fill {
			if parent >= 0 && parent < unbounded {
				parent
			} else {
				unknown
			}
		}
		.auto {
			unknown
		}
	}
}

pub enum Display as u8 {
	flex
	grid
	none
}

pub enum Position as u8 {
	relative
	absolute
}

pub enum Direction as u8 {
	row
	column
	row_reverse
	column_reverse
}

pub enum Justify as u8 {
	start
	end
	center
	between
	around
	evenly
}

pub enum Align as u8 {
	stretch
	start
	end
	center
}

pub enum AlignSelf as u8 {
	auto
	stretch
	start
	end
	center
}

pub enum Overflow as u8 {
	visible
	hidden
	scroll
}

// Margin sides that are `auto` (flex auto margins).
@[flag]
pub enum AutoMargin as u8 {
	top
	right
	bottom
	left
}

pub struct Style {
pub mut:
	display      Display
	position     Position
	direction    Direction
	wrap         bool
	justify      Justify
	align_items  Align
	align_self   AlignSelf
	gap_row      f32
	gap_col      f32
	grow         f32
	shrink       f32 = 1
	basis        Length
	width        Length
	height       Length
	min_width    Length
	min_height   Length
	max_width    Length
	max_height   Length
	aspect_ratio f32 // width / height; 0 = none
	padding      prim.Edges
	margin       prim.Edges
	margin_auto  AutoMargin
	border       prim.Edges
	inset_top    Length
	inset_right  Length
	inset_bottom Length
	inset_left   Length
	overflow_x   Overflow
	overflow_y   Overflow
	grid         &GridStyle = unsafe { nil } // nil: not a grid container or item (most nodes)
}

// grid_style is the grid part of the style (the defaults when none was set).
pub fn (s &Style) grid_style() GridStyle {
	return if s.grid == unsafe { nil } { GridStyle{} } else { *s.grid }
}

pub fn (s &Style) is_row() bool {
	return s.direction == .row || s.direction == .row_reverse
}

pub fn (s &Style) is_reverse() bool {
	return s.direction == .row_reverse || s.direction == .column_reverse
}

// frame is padding plus border: the space between the border box and the content box.
pub fn (s &Style) frame() prim.Edges {
	return prim.Edges{s.padding.top + s.border.top, s.padding.right + s.border.right,
		s.padding.bottom + s.border.bottom, s.padding.left + s.border.left}
}
