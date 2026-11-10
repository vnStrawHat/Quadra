module main

import quadra.kit.term
import quadra.testing

fn test_a_second_is_described_from_two_readings() {
	a := term.Stats{
		bytes:    1048576
		chunks:   10
		parse_us: 1000
		frames:   5
		paint_us: 2000
	}
	b := term.Stats{
		bytes:    3145728
		chunks:   20
		parse_us: 6000
		frames:   65
		paint_us: 122000
	}
	assert stats_line(a, b, 8500) == '2.0 MiB/s  10 chunks/s  parse 0.50 ms  60 fps  paint 2.00 ms  frame 8.50 ms'
	assert stats_line(a, a, 0).contains('parse 0.00 ms  0 fps  paint 0.00 ms')
}

fn test_the_status_bar_shows_the_stats_only_when_asked() {
	mut h := testing.new_harness()!
	_, mut one := prepare(mut h.app, '.tmp/oneterm-test')!
	mut fake := &term.FakePty{}
	one.spawn = fake.spawner()
	one.stats_on = true
	h.open(one, width: 1000, height: 700)!
	h.advance(1100) // the first sample only sets the baseline
	fake.output('hello')
	h.advance(1000)
	assert h.texts().any(it.contains('MiB/s') && it.contains('chunks/s'))
}
