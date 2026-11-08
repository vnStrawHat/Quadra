// Div builder: CSS grid.
module ui

import quadra.layout

pub fn (self_ &Div) grid() &Div {
	mut d := unsafe { self_ }
	d.style.display = .grid
	return d
}

pub fn (self_ &Div) cols(tracks []layout.Track) &Div {
	mut d := unsafe { self_ }
	d.style.display = .grid
	d.grid_mut().columns = tracks
	return d
}

pub fn (self_ &Div) rows(tracks []layout.Track) &Div {
	mut d := unsafe { self_ }
	d.grid_mut().rows = tracks
	return d
}

pub fn (self_ &Div) auto_rows(t layout.Track) &Div {
	mut d := unsafe { self_ }
	d.grid_mut().auto_rows = t
	return d
}

pub fn (self_ &Div) areas(rows []string) &Div {
	mut d := unsafe { self_ }
	d.style.display = .grid
	d.grid_mut().areas = rows
	return d
}

pub fn (self_ &Div) area(name string) &Div {
	mut d := unsafe { self_ }
	d.grid_mut().area = name
	return d
}

pub fn (self_ &Div) col(start int, span int) &Div {
	mut d := unsafe { self_ }
	d.grid_mut().col_start, d.grid_mut().col_span = start, span
	return d
}

pub fn (self_ &Div) row(start int, span int) &Div {
	mut d := unsafe { self_ }
	d.grid_mut().row_start, d.grid_mut().row_span = start, span
	return d
}

pub fn (self_ &Div) col_span(n int) &Div {
	mut d := unsafe { self_ }
	d.grid_mut().col_span = n
	return d
}

// grid_fill makes as many columns of at least min px as fit (repeat(auto-fill, minmax(min, 1fr))).
pub fn (self_ &Div) grid_fill(min f32) &Div {
	mut d := unsafe { self_ }
	d.style.display = .grid
	d.grid_mut().fill_min = min
	return d
}

// grid_mut is the div's grid style, made on first use: most divs never need one.
fn (mut d Div) grid_mut() &layout.GridStyle {
	if d.style.grid == unsafe { nil } {
		d.style.grid = &layout.GridStyle{}
	}
	return d.style.grid
}
