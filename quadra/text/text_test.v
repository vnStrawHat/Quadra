module text

fn test_nfc_vietnamese() {
	// e + combining circumflex + combining dot below, in either order -> ệ
	assert normalize_nfc('Việt') == 'Việt'
	assert normalize_nfc('Việt') == 'Việt'
	assert normalize_nfc('Tiếng') == 'Tiếng'
	assert normalize_nfc('à') == 'à'
	assert normalize_nfc('Việt Nam') == 'Việt Nam' // already NFC
	assert normalize_nfc('plain ascii') == 'plain ascii'
	assert normalize_nfc('x́') == 'x́' // no precomposed form: kept
}

fn test_grapheme_boundaries() {
	assert boundaries('ab') == [0, 1, 2]
	nfd := 'ệ' // one cluster of 5 bytes
	assert boundaries(nfd) == [0, 5]
	assert boundaries('Việt') == [0, 1, 2, 5, 6]
	family := '👨‍👩‍👧' // ZWJ sequence: one cluster
	assert boundaries(family).len == 2
	flags := '🇻🇳🇺🇸' // two flags
	assert boundaries(flags).len == 3
	assert boundaries('👍🏽!').len == 3 // skin tone joins its emoji
	assert boundaries('a\r\nb') == [0, 1, 3, 4]
	assert prev_grapheme('Việt', 5) == 2
	assert next_grapheme('Việt', 2) == 5
}

fn test_fold_diacritics() {
	assert fold_diacritics('Hải Phòng') == 'Hai Phong'
	assert fold_diacritics('Đà Nẵng, Việt Nam') == 'Da Nang, Viet Nam'
	assert fold_diacritics('ệ') == 'e'
}

fn test_decode_matches_runes() {
	for s in ['abc', 'Việt Nam', 'ệ', '👨‍👩‍👧', '🇻🇳', '€', 'x\xe2\x82',
		'\x80a', '\xc3'] {
		mut i := 0
		for i < s.len {
			r, n := decode(s, i)
			end := if i + utf8_char_len(s[i]) > s.len { s.len } else { i + utf8_char_len(s[i]) }
			want := s[i..end].runes()
			assert n == end - i
			assert r == if want.len > 0 {
				want[0]
			} else {
				rune(s[i])
			}
			i += n
		}
	}
}
