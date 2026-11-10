module term

import quadra.platform

fn enc(k platform.Key, mods platform.Mods, app bool) string {
	return (encode_key(k, mods, app) or { return '<none>' }).bytestr()
}

fn test_key_encoding() {
	none_ := platform.Mods.zero()
	assert enc(.enter, none_, false) == '\r'
	assert enc(.backspace, none_, false) == '\x7f'
	assert enc(.backspace, .ctrl, false) == '\x08'
	assert enc(.tab, .shift, false) == '\x1b[Z'
	assert enc(.up, none_, false) == '\x1b[A'
	assert enc(.up, none_, true) == '\x1bOA' // application cursor keys
	assert enc(.left, .ctrl, true) == '\x1b[1;5D'
	assert enc(.right, platform.Mods.shift | .alt, false) == '\x1b[1;4C'
	assert enc(.home, none_, false) == '\x1b[H'
	assert enc(.delete, none_, false) == '\x1b[3~'
	assert enc(.page_down, .ctrl, false) == '\x1b[6;5~'
	assert enc(.f1, none_, false) == '\x1bOP'
	assert enc(.f4, .shift, false) == '\x1b[1;2S'
	assert enc(.f12, none_, false) == '\x1b[24~'
	assert enc(.c, .ctrl, false) == '\x03'
	assert enc(.a, platform.Mods.ctrl | .alt, false) == '\x1b\x01'
	assert enc(.bracket_left, .ctrl, false) == '\x1b'
	assert enc(.space, .ctrl, false) == '\x00'
	assert enc(.b, .alt, false) == '\x1bb'
	assert enc(.b, platform.Mods.alt | .shift, false) == '\x1bB'
	assert enc(.a, none_, false) == '<none>' // plain text comes from text input
	assert enc(.a, .shift, false) == '<none>'
}

fn test_paste() {
	assert paste_bytes('a\nb', false).bytestr() == 'a\rb'
	assert paste_bytes('x\r\ny', true).bytestr() == '\x1b[200~x\r\ny\x1b[201~'
	assert paste_bytes('a\x1bb\x03c', true).bytestr() == '\x1b[200~abc\x1b[201~'
	// An embedded end marker cannot break out of the bracket.
	assert paste_bytes('a\x1b[201~rm', true).bytestr() == '\x1b[200~arm\x1b[201~'
}
