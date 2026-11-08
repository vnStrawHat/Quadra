module viz

fn test_nice_max() {
	assert nice_max(0) == 1
	assert nice_max(7) == 10
	assert nice_max(180) == 200
	assert nice_max(230) == 250
	assert nice_max(0.37) == 0.5
	assert nice_max(1000) == 1000
}

fn test_line_spans_cover_the_line() {
	// A ramp from 0 to 100 over 10 columns, 50 px tall, max 100.
	spans := line_spans([0.0, 100], 10, 50, 100, 2)
	assert spans.len == 10
	assert spans[0].y == 47.5 // the first column's center
	for x, sp in spans {
		assert sp.bottom - sp.top >= 2 // never thinner than the line
		assert sp.top <= 50 - f32(x + 1) * 5 + 1 // and it reaches the next column's height
		assert sp.thick > 2 * 5.09 && sp.thick < 2 * 5.1 // thick 2 at slope 5: 2 * sqrt(26) tall
		assert sp.bottom - sp.top - sp.thick > 4.99 // and it travels 5 down the column
	}
	// A flat series is one line high everywhere; values are clamped to the plot.
	flat := line_spans([50.0, 50, 50], 4, 50, 100, 1)
	assert flat.all(it.top == 24.5 && it.bottom == 25.5 && it.thick == 1)
	over := line_spans([500.0], 2, 50, 100, 1)
	assert over[0].y == 0
	assert line_spans([]f64{}, 10, 50, 100, 1).len == 0
}

fn test_values_and_labels() {
	assert value_at([0.0, 10, 20], 0.25) == 5
	assert value_at([0.0, 10, 20], 1) == 20
	assert bar_width(50, 200, 100) == 25
	assert short(1500) == '1.5k' && short(2000) == '2k' && short(250) == '250'
	assert short(0.25) == '0.25'
	assert hh_mm(1793966400000 + 90 * 60_000) == '13:30'
}
