// Themes as JSON (specs/15). A theme file names its mode and lists colors under the
// token names of the spec's tables; it starts from the built-in theme of its mode, so a
// new theme lists only what differs. The built-in themes are such files (themes/).
module kit

import os
import strconv
import x.json2
import quadra.prim

const theme_keys = ['name', 'mode', 'palette', 'syntax', 'editor', 'terminal', 'series']

// load_theme reads a theme file (theme_from_json).
pub fn load_theme(path string) !Theme {
	return theme_from_json(os.read_file(path)!) or { return error('${path}: ${err}') }
}

// theme_from_json parses a theme: {"name", "mode": "dark" | "light" | "high_contrast",
// "palette": {...}, "syntax": {...}, "editor": {...}, "terminal": {..., "ansi": [16]},
// "series": [...]}, each color "#RRGGBB" or "#RRGGBBAA". Colors left out keep the values
// of dark() or light(), whichever matches the mode.
pub fn theme_from_json(text string) !Theme {
	o, mode := theme_object(text)!
	base := if mode == .light { light() } else { dark() }
	return base.merged(o, mode)
}

// built_in parses a bundled theme, which lists every color.
fn built_in(text string) Theme {
	o, mode := theme_object(text) or { panic('a built-in theme: ${err}') }
	return Theme{}.merged(o, mode) or { panic('a built-in theme: ${err}') }
}

fn theme_object(text string) !(map[string]json2.Any, Mode) {
	raw := json2.decode[json2.Any](text)!
	if raw !is map[string]json2.Any {
		return error('a theme is a JSON object')
	}
	o := raw.as_map()
	for k, _ in o {
		if k !in theme_keys {
			return error('${k}: not a theme key (${theme_keys.join(', ')})')
		}
	}
	mode := match (o['mode'] or { json2.Any('') }).str() {
		'dark' { Mode.dark }
		'light' { Mode.light }
		'high_contrast' { Mode.high_contrast }
		else { return error('mode: "dark", "light" or "high_contrast"') }
	}

	return o, mode
}

fn (base Theme) merged(o map[string]json2.Any, mode Mode) !Theme {
	mut t := base
	t.mode = mode
	if name := o['name'] {
		t.name = name.str()
	}
	colors_into(mut t.p, section(o, 'palette')!, 'palette')!
	colors_into(mut t.code.syntax, section(o, 'syntax')!, 'syntax')!
	colors_into(mut t.code.editor, section(o, 'editor')!, 'editor')!
	term := section(o, 'terminal')!
	colors_into(mut t.code.terminal, term, 'terminal')!
	if a := term['ansi'] {
		ansi := color_list(a, 'terminal.ansi')!
		if ansi.len != 16 {
			return error('terminal.ansi: 16 colors')
		}
		for i, c in ansi {
			t.code.terminal.ansi[i] = c
		}
	}
	if s := o['series'] {
		t.series = color_list(s, 'series')!
	}
	if t.series.len == 0 {
		return error('series: at least one color')
	}
	return t.finish()
}

fn section(o map[string]json2.Any, key string) !map[string]json2.Any {
	v := o[key] or { return map[string]json2.Any{} }
	if v !is map[string]json2.Any {
		return error('${key}: an object of colors')
	}
	return v.as_map()
}

// colors_into sets the color fields of s named by the keys of o.
fn colors_into[T](mut s T, o map[string]json2.Any, at string) ! {
	for k, v in o {
		mut found := false
		$for f in T.fields {
			if f.name == k {
				found = true
				$if f.typ is prim.Color {
					s.$(f.name) = color_of(v, '${at}.${k}')!
				}
			}
		}
		if !found {
			return error('${at}.${k}: not a color of ${at}')
		}
	}
}

fn color_list(v json2.Any, at string) ![]prim.Color {
	if v !is []json2.Any {
		return error('${at}: a list of colors')
	}
	mut out := []prim.Color{}
	for i, c in v.as_array() {
		out << color_of(c, '${at}[${i}]')!
	}
	return out
}

fn color_of(v json2.Any, at string) !prim.Color {
	s := v.str()
	if v !is string || (s.len != 7 && s.len != 9) || s[0] != `#` {
		return error('${at}: a color is "#RRGGBB" or "#RRGGBBAA", not ${v.json_str()}')
	}
	n := u32(strconv.parse_uint(s[1..], 16, 32) or { return error('${at}: bad hex ${s}') })
	return if s.len == 7 { prim.rgb(n) } else { prim.rgba(n) }
}

// to_json writes the theme in the file format, every color included: a starting point
// for a new theme.
pub fn (t Theme) to_json() string {
	mut term := color_lines(t.code.terminal)
	term << '"ansi": [${t.code.terminal.ansi[..].map(quoted_hex(it)).join(', ')}]'
	parts := [
		'"name": ${json2.encode(t.name)}',
		'"mode": "${t.mode}"',
		block('palette', color_lines(t.p)),
		block('syntax', color_lines(t.code.syntax)),
		block('editor', color_lines(t.code.editor)),
		block('terminal', term),
		'"series": [${t.series.map(quoted_hex(it)).join(', ')}]',
	]
	return '{\n\t' + parts.join(',\n\t') + '\n}\n'
}

fn color_lines[T](s T) []string {
	mut out := []string{}
	$for f in T.fields {
		$if f.typ is prim.Color {
			c := s.$(f.name)
			out << '"${f.name}": ${quoted_hex(c)}'
		}
	}
	return out
}

fn block(key string, lines []string) string {
	return '"${key}": {\n\t\t' + lines.join(',\n\t\t') + '\n\t}'
}

fn quoted_hex(c prim.Color) string {
	h := c.hex().to_upper()
	return '"' + (if h.ends_with('FF') {
		h[..7]
	} else {
		h
	}) + '"'
}
