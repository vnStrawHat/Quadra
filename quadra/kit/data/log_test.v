module data

fn entries(n int) []LogEntry {
	return []LogEntry{len: n, init: LogEntry{i64(index) * 1000, if index % 10 == 3 {
		'\x1b[31mERROR\x1b[0m request ${index} failed'
	} else {
		'INFO request ${index} ok'
	}}}
}

fn test_store_packs_lines() {
	mut s := LogStore{}
	s.push(5, 'one')
	s.push(6, '')
	s.push(7, 'three')
	assert s.len() == 3
	assert s.line(0) == 'one' && s.line(1) == '' && s.line(2) == 'three'
	assert s.at(2) == 7
	s.clear()
	assert s.len() == 0
}

fn test_filter_hide_dim_and_regex() {
	mut lv := LogView{}
	lv.append(entries(100))
	assert lv.shown_count() == 100
	lv.set_filter('error', false) // case-insensitive, escapes not matched
	assert lv.shown_count() == 10
	assert lv.shown_line(0) == 3
	lv.append(entries(14)) // appends are filtered as they arrive
	assert lv.shown_count() == 12
	lv.set_filter(r'request \d+5 ', true)
	assert lv.shown_count() == 9 // 15, 25, ... 95 (the pattern needs a digit before the 5)
	lv.mode = .dim
	lv.set_filter('error', false)
	assert lv.shown_count() == 114 // dim lists every line
	lv.mode = .hide
	lv.set_filter('(', true)
	assert lv.error != '' && lv.shown_count() == 114 // a bad pattern filters nothing
	assert strip_escapes('\x1b[1;32mok\x1b[0m!') == 'ok!'
}

fn test_clock() {
	assert clock(1793966400000 + 3_723_045) == '13:02:03.045'
}
