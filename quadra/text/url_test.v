// The shared URL detector (specs/04, URLs): punctuation, parentheses, schemes, byte ranges.
module text

fn urls(s string) []string {
	return find_urls(s).map(it.url)
}

fn test_trailing_punctuation_ends_a_url() {
	assert urls('see https://a.b/c. Then https://d.e/f, and "https://g.h"!') == [
		'https://a.b/c',
		'https://d.e/f',
		'https://g.h',
	]
	assert urls('<https://a.b/x> and `http://c.d/y`') == ['https://a.b/x', 'http://c.d/y']
	assert urls('https://. https://') == [] // a prefix alone is no URL
}

fn test_parentheses_stay_balanced() {
	assert urls('(see https://en.wikipedia.org/wiki/Foo_(bar))') == [
		'https://en.wikipedia.org/wiki/Foo_(bar)',
	]
	assert urls('(https://a.b/c).') == ['https://a.b/c']
	// The terminal uses the same rules (specs/22): the balanced `)` stays there too.
	chars := 'https://w.org/Foo_(bar)'.runes()
	plen, end := url_run(chars, 0)
	assert chars[..url_trim(chars, plen, end)].string() == 'https://w.org/Foo_(bar)'
}

fn test_only_web_and_mail_schemes_open() {
	assert url_openable('https://a.b') && url_openable('HTTP://a.b') && url_openable('mailto:x@y.z')
	for bad in ['file:///C:/Windows/system32/calc.exe', 'ms-settings:privacy', 'javascript:alert(1)',
		'myapp://run', 'ftp://a.b/x', 'https://a.b/\u202egnp', 'https://a.b/x y', 'https://a\x07'] {
		assert !url_openable(bad), bad
	}
	// No caller detects ftp or a scheme glued to a word.
	assert urls('ftp://a.b/x xhttps://c.d www.e.org') == ['https://www.e.org']
}

fn test_byte_ranges_cover_non_ascii_text() {
	s := 'Đọc https://d.org/đường/路 ngay'
	sp := find_urls(s)
	assert sp.len == 1
	assert s[sp[0].start..sp[0].end] == 'https://d.org/đường/路'
	assert find_urls('no links here, just a:b').len == 0
}

// Regression (found by quadra/kit/md/fuzz_test.v): a cut UTF-8 sequence before a URL. runes()
// makes one U+FFFD per bad byte, but the byte offsets stepped by the lead byte's length, so
// the range ran past the line and slicing it panicked.
fn test_invalid_utf8_before_a_url_keeps_ranges_in_the_line() {
	for bad in [[u8(0xe2), 0x82], [u8(0xf0), 0x9f], [u8(0xff)],
		[u8(0xef), 0xbf, 0xbd]] {
		mut b := 'a '.bytes()
		b << bad
		b << 'tp: http://bare.example/x?y=1.'.bytes()
		s := b.bytestr()
		urls := find_urls(s)
		assert urls.len == 1
		assert s[urls[0].start..urls[0].end] == 'http://bare.example/x?y=1'
	}
}
