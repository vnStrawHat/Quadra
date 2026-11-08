module code

fn breaks(s string, cols int) []int {
	mut out := []int{}
	row_breaks(display_line(s, 4).shown, cols, mut out)
	return out
}

fn test_row_breaks() {
	assert breaks('', 4) == [0]
	assert breaks('abc', 4) == [0]
	assert breaks('aaaa bbbb', 4) == [0, 5] // the space hangs at the edge
	assert breaks('hello world foo', 11) == [0, 12]
	assert breaks('one two three', 8) == [0, 8] // after the last space that fits
	assert breaks('abcdefghij', 4) == [0, 4, 8] // a long word breaks inside
	assert breaks('\tab cd', 6) == [0, 7] // tabs count as their spaces
	assert breaks('ééé ééé', 3) == [0, 7] // columns are characters, not bytes
}
