// Display lines: a document line with tabs expanded to the next tab stop, and the mapping
// between source and display byte offsets (tokens, carets and clicks go through it).
module code

struct DisplayLine {
	src   string
	shown string
	dcol  []int // display offset of each source byte, len src.len + 1; empty without tabs
}

fn display_line(s string, tab int) DisplayLine {
	if !s.contains('\t') {
		return DisplayLine{
			src:   s
			shown: s
		}
	}
	mut out := []u8{cap: s.len + 16}
	mut dcol := []int{len: s.len + 1}
	mut col := 0
	for i := 0; i < s.len; i++ {
		dcol[i] = out.len
		c := s[i]
		if c == `\t` {
			n := tab - col % tab
			for _ in 0 .. n {
				out << ` `
			}
			col += n
			continue
		}
		out << c
		if c < 0x80 || c >= 0xC0 { // count runes, not continuation bytes
			col++
		}
	}
	dcol[s.len] = out.len
	return DisplayLine{
		src:   s
		shown: out.bytestr()
		dcol:  dcol
	}
}

// to_shown maps a source offset in the line to a display offset.
fn (l &DisplayLine) to_shown(i int) int {
	k := clamp(i, 0, l.src.len)
	return if l.dcol.len == 0 { k } else { l.dcol[k] }
}

// to_src maps a display offset back to the source offset at or before it.
fn (l &DisplayLine) to_src(d int) int {
	if l.dcol.len == 0 {
		return clamp(d, 0, l.src.len)
	}
	mut lo, mut hi := 0, l.src.len
	for lo < hi {
		mid := (lo + hi + 1) / 2
		if l.dcol[mid] <= d {
			lo = mid
		} else {
			hi = mid - 1
		}
	}
	// A click in the second half of a tab's spaces goes after the tab.
	if lo < l.src.len && l.src[lo] == `\t` && d - l.dcol[lo] > (l.dcol[lo + 1] - l.dcol[lo]) / 2 {
		return lo + 1
	}
	return lo
}
