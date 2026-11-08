// String tables per locale with plurals and named arguments (specs/10). The current locale
// is a global model: switching language re-renders exactly the views that read strings.
module app

import x.json2
import quadra.core

pub struct I18n {
pub mut:
	locale   string = 'en'
	fallback string = 'en'
	tables   map[string]map[string]string
	log      &Logger = unsafe { nil }
}

// install_i18n parses JSON string tables ({"files.count.one": "{n} file", ...}), keyed by
// locale code, and makes `locale` current. Nested objects flatten to dotted keys.
pub fn install_i18n(mut cx core.Cx, sources map[string]string, locale string, fallback string, log &Logger) ! {
	mut tables := map[string]map[string]string{}
	for code, text in sources {
		mut flat := map[string]string{}
		flatten('', json2.decode[json2.Any](text) or { return error('i18n/${code}.json: ${err}') }, mut
			flat)
		tables[code] = flat.move()
	}
	cx.set_global(I18n{
		locale:   locale
		fallback: fallback
		tables:   tables
		log:      unsafe { log }
	})
}

fn flatten(prefix string, v json2.Any, mut out map[string]string) {
	if v is map[string]json2.Any {
		for k, child in v {
			flatten(if prefix == '' { k } else { '${prefix}.${k}' }, child, mut out)
		}
	} else {
		out[prefix] = v.str()
	}
}

// set_locale switches the language; views that read strings re-render.
pub fn set_locale(mut cx core.Cx, code string) {
	mut i := cx.peek_global[I18n]() or { return }
	i.locale = code
	cx.touch_global[I18n]()
}

// locale returns the current locale code (tracked).
pub fn locale(cx &core.Cx) string {
	i := cx.global[I18n]() or { return 'en' }
	return i.locale
}

// t looks a key up in the current locale, then the fallback, then returns the key itself.
pub fn t(cx &core.Cx, key string) string {
	i := cx.global[I18n]() or { return key }
	return i.lookup(key)
}

// tn picks the plural form for n (`key.one` / `key.other`) and fills `{n}`.
pub fn tn(cx &core.Cx, key string, n int) string {
	i := cx.global[I18n]() or { return key }
	form := plural_form(i.locale, n)
	s := i.lookup_opt('${key}.${form}') or { i.lookup('${key}.other') }
	return s.replace('{n}', format_int(i.locale, n))
}

// tf fills named arguments: tf(cx, 'note.saved', {'title': t}) for "Saved {title}".
pub fn tf(cx &core.Cx, key string, args map[string]string) string {
	mut s := t(cx, key)
	for k, v in args {
		s = s.replace('{${k}}', v)
	}
	return s
}

// plural_form implements the CLDR cardinal rules of the shipped locales.
pub fn plural_form(locale string, n int) string {
	return match locale.all_before('-') {
		'vi', 'ja', 'zh', 'ko' {
			'other'
		}
		else {
			if n == 1 {
				'one'
			} else {
				'other'
			}
		}
	}
}

fn (i &I18n) lookup(key string) string {
	return i.lookup_opt(key) or {
		if i.log != unsafe { nil } {
			mut l := i.log
			l.warn_once('i18n:${i.locale}:${key}', 'i18n', 'missing key "${key}" in ${i.locale}')
		}
		key
	}
}

fn (i &I18n) lookup_opt(key string) ?string {
	if s := i.tables[i.locale][key] {
		return s
	}
	if s := i.tables[i.fallback][key] {
		return s
	}
	return none
}
