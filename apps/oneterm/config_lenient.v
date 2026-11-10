// The settings of ui_config.json read leniently (specs/22, Config).
module main

import math
import x.json2

// lenient_ui_document repairs the two settings that are read leniently, so one bad value costs
// that setting and not the whole file. A clean document comes back untouched.
fn lenient_ui_document(text string) string {
	raw := json2.decode[json2.Any](text) or { return text }
	mut root := any_map(raw) or { return text }
	mut repaired := false
	if value := root['lock_after_idle_minutes'] {
		have := any_f64(value) or { -1 }
		good := clean_minutes(value)
		if good == 0 {
			root.delete('lock_after_idle_minutes')
			repaired = true
		} else if good != have {
			root['lock_after_idle_minutes'] = json2.Any(good)
			repaired = true
		}
	}
	if tabs := root['vertical_tabs'] {
		if !is_clean_vertical_tabs(tabs) {
			root['vertical_tabs'] = lenient_vertical_tabs(tabs)
			repaired = true
		}
	}
	return if repaired { json2.encode(root) } else { text }
}

fn any_map(v json2.Any) ?map[string]json2.Any {
	if v is map[string]json2.Any {
		return v.clone()
	}
	return none
}

fn any_f64(v json2.Any) ?f64 {
	if v is f64 {
		return f64(v)
	}
	return none
}

fn any_bool(v json2.Any) ?bool {
	if v is bool {
		return bool(v)
	}
	return none
}

// clean_minutes is a whole non-negative number clamped to 1-480, 0 for anything else.
fn clean_minutes(v json2.Any) f64 {
	m := any_f64(v) or { return 0 }
	if m < 0 || m != math.floor(m) {
		return 0
	}
	return if m < 1 {
		1
	} else if m > 480 {
		480
	} else {
		m
	}
}

fn is_clean_vertical_tabs(v json2.Any) bool {
	m := any_map(v) or { return false }
	for key in ['enabled', 'collapsed'] {
		if flag := m[key] {
			any_bool(flag) or { return false }
		}
	}
	if value := m['width'] {
		width := any_f64(value) or { return false }
		return width == math.floor(width)
	}
	return true
}

// lenient_vertical_tabs keeps the fields of the right type; the others are their defaults.
fn lenient_vertical_tabs(v json2.Any) json2.Any {
	mut out := map[string]json2.Any{}
	m := any_map(v) or { return json2.Any(out) }
	for key in ['enabled', 'collapsed'] {
		if flag := m[key] {
			if on := any_bool(flag) {
				out[key] = json2.Any(on)
			}
		}
	}
	if value := m['width'] {
		if width := any_f64(value) {
			if !math.is_nan(width) && !math.is_inf(width, 0) {
				out['width'] = json2.Any(math.round(width))
			}
		}
	}
	return json2.Any(out)
}
