// Styles per class, spans for painting, and the per-view overlay (specs/24).
module highlight

import quadra.prim
import x.json2

// ClassStyle is one class's resolved style.
pub struct ClassStyle {
pub mut:
	has_fg        bool
	fg            prim.Color
	has_bg        bool
	bg            prim.Color
	bold          bool
	italic        bool
	underline     bool
	override_ansi bool // force the class fg over an explicit ANSI fg (off by default)
}

// ClassStyles is a flat array indexed by class byte.
pub struct ClassStyles {
pub mut:
	styles [class_count]ClassStyle
}

pub fn (s &ClassStyles) style(class u8) ClassStyle {
	return if class < class_count { s.styles[class] } else { ClassStyle{} }
}

pub fn (mut s ClassStyles) set_fg(class Class, color prim.Color) {
	s.styles[u8(class)].has_fg = true
	s.styles[u8(class)].fg = color
}

// is_active: are any colors configured? All empty means the layer is a no-op.
pub fn (s &ClassStyles) is_active() bool {
	for x in s.styles {
		if x.has_fg || x.has_bg {
			return true
		}
	}
	return false
}

const class_keys = {
	'promptSign':  Class.prompt_sign
	'command':     .command
	'option':      .option
	'error':       .error
	'success':     .success
	'warn':        .warn
	'info':        .info
	'debug':       .debug
	'path':        .path
	'ip':          .ip
	'mac':         .mac
	'dateTime':    .date_time
	'number':      .number
	'string':      .quoted
	'operator':    .operator
	'bracket':     .bracket
	'url':         .url
	'permission':  .permission
	'permType':    .perm_type
	'permRead':    .perm_read
	'permWrite':   .perm_write
	'permExec':    .perm_exec
	'permSpecial': .perm_special
	'permNone':    .perm_none
}

// parse_color reads #RRGGBB, #RGB, with or without the `#`.
pub fn parse_color(hex string) ?prim.Color {
	h := hex.trim_left('#')
	if h.len == 3 {
		return prim.parse_hex('#${h[0..1]}${h[0..1]}${h[1..2]}${h[1..2]}${h[2..3]}${h[2..3]}')
	}
	return prim.parse_hex('#${h}')
}

const default_styles = parse_semantic_json($embed_file('assets/default.json').to_string())

// load_default_styles is the shipped default semantic style.
pub fn load_default_styles() ClassStyles {
	return default_styles
}

// parse_semantic_json reads `{"styles": {key: {"foreground", "background", "decoration",
// "bold", "italic"}}}`. Invalid JSON gives the empty style; unknown keys are ignored.
pub fn parse_semantic_json(text string) ClassStyles {
	mut styles := ClassStyles{}
	doc := json2.decode[json2.Any](text) or { return styles }
	if doc !is map[string]json2.Any {
		return styles
	}
	root := doc.as_map()
	o := root['styles'] or { return styles }
	for key, val in o.as_map() {
		class := class_keys[key] or { continue }
		v := val.as_map()
		idx := u8(class)
		if fg := v['foreground'] {
			if c := parse_color(fg.str()) {
				styles.styles[idx].has_fg = true
				styles.styles[idx].fg = c
			}
		}
		if bg := v['background'] {
			if c := parse_color(bg.str()) {
				styles.styles[idx].has_bg = true
				styles.styles[idx].bg = c
			}
		}
		if deco := v['decoration'] {
			styles.styles[idx].underline = deco.str() == 'underline'
		}
		if b := v['bold'] {
			styles.styles[idx].bold = b.bool()
		}
		if i := v['italic'] {
			styles.styles[idx].italic = i.bool()
		}
	}
	return styles
}

// Span is one styled run of a line: rune indices, end exclusive.
pub struct Span {
pub:
	start int
	end   int
	class Class
	style ClassStyle
}

// spans_into merges runs of equal non-plain classes into styled spans.
pub fn spans_into(classes []u8, styles &ClassStyles, mut out []Span) {
	out.clear()
	mut i := 0
	for i < classes.len {
		c := classes[i]
		mut j := i + 1
		for j < classes.len && classes[j] == c {
			j++
		}
		if c != u8(Class.plain) {
			out << Span{i, j, class_from_u8(c), styles.style(c)}
		}
		i = j
	}
}

pub fn spans(classes []u8, styles &ClassStyles) []Span {
	mut out := []Span{}
	spans_into(classes, styles, mut out)
	return out
}

// highlight_row scans one line and returns its styled runs.
pub fn highlight_row(line string, profile ShellProfile, role RowRole, input_at int, styles &ClassStyles) []Span {
	return spans(scan_line(line, profile, role, input_at), styles)
}

// Overlay is the per-view gate: the profile, the setting and the last exit code.
pub struct Overlay {
pub mut:
	profile       ShellProfile
	exit_code     int // of the most recently completed block; meaningless when has_exit_code is false
	has_exit_code bool
	enabled       bool = true
}

pub fn new_overlay(profile ShellProfile, enabled bool) Overlay {
	return Overlay{
		profile: profile
		enabled: enabled
	}
}

// scan_into classifies one logical line; disabled, every class is plain.
pub fn (o &Overlay) scan_into(line string, role RowRole, input_at int, mut scratch ScanScratch, mut out []u8) {
	if !o.enabled {
		out.clear()
		for _ in 0 .. line.runes().len {
			out << 0
		}
		return
	}
	scan_line_into(line, o.profile, role, input_at, mut scratch, mut out)
}

// scan_runes_into is scan_into for a decoded line; it allocates nothing once out has grown.
pub fn (o &Overlay) scan_runes_into(chars []rune, role RowRole, input_at int, mut scratch ScanScratch, mut out []u8) {
	if !o.enabled {
		out.clear()
		for _ in 0 .. chars.len {
			out << 0
		}
		return
	}
	scan_runes_into(chars, o.profile, role, input_at, mut scratch, mut out)
}
