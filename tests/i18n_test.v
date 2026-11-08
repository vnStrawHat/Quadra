// i18n: lookups, fallback, plurals, re-render on locale switch, vi/en formatting.
module main

import time
import quadra.app
import quadra.core
import quadra.testing
import quadra.ui

const en_json = '{"title": "Notes", "files": {"count": {"one": "{n} file", "other": "{n} files"}}, "saved": "Saved {name}", "only_en": "English only"}'
const vi_json = '{"title": "Ghi chú", "files": {"count": {"other": "{n} tệp"}}, "saved": "Đã lưu {name}"}'

struct Title {}

@[markused]
pub fn (mut v Title) render(mut cx core.ViewCx[Title]) core.Element {
	return ui.text(app.t(cx.Cx, 'title'))
}

struct Other {}

@[markused]
pub fn (mut v Other) render(mut cx core.ViewCx[Other]) core.Element {
	return ui.text('static')
}

struct Both {
mut:
	title core.View[Title]
	other core.View[Other]
}

@[markused]
pub fn (mut b Both) render(mut cx core.ViewCx[Both]) core.Element {
	if b.title.id == 0 {
		b.title = cx.new_view(Title{})
		b.other = cx.new_view(Other{})
	}
	return ui.div().child(cx.child_view(b.title)).child(cx.child_view(b.other))
}

fn test_lookup_plurals_fallback_and_missing() {
	mut h := testing.new_harness()!
	mut cx := core.Cx{
		app: h.app
	}
	mut log := app.new_logger('', 'test', .info)
	app.install_i18n(mut cx, {
		'en': en_json
		'vi': vi_json
	}, 'en', 'en', log)!
	assert app.tn(cx, 'files.count', 1) == '1 file'
	assert app.tn(cx, 'files.count', 1234) == '1,234 files'
	assert app.tf(cx, 'saved', {
		'name': 'a.md'
	}) == 'Saved a.md'
	app.set_locale(mut cx, 'vi')
	assert app.tn(cx, 'files.count', 1) == '1 tệp' // Vietnamese has one plural form
	assert app.tn(cx, 'files.count', 1234) == '1.234 tệp'
	assert app.t(cx, 'only_en') == 'English only' // falls back to en
	assert app.t(cx, 'nope') == 'nope'
	app.t(cx, 'nope')
	assert log.recent(10).filter(it.msg.contains('nope')).len == 1 // logged once
}

fn test_switching_locale_rerenders_only_readers() {
	mut h := testing.new_harness()!
	mut cx := core.Cx{
		app: h.app
	}
	app.install_i18n(mut cx, {
		'en': en_json
		'vi': vi_json
	}, 'en', 'en', unsafe { nil })!
	v := h.open(Both{})!
	b := cx.peek(v.model())
	assert h.has_text('Notes')
	t0, o0 := h.renders_of(b.title), h.renders_of(b.other)
	app.set_locale(mut cx, 'vi')
	h.frame()
	assert h.has_text('Ghi chú')
	assert h.renders_of(b.title) == t0 + 1
	assert h.renders_of(b.other) == o0
}

fn test_formatting() {
	assert app.format_number('vi', 1234567.891, 2) == '1.234.567,89'
	assert app.format_number('en', -1234.5, 1) == '-1,234.5'
	assert app.format_number('en', 999, 0) == '999'
	assert app.format_size('vi', 1536 * 1024) == '1,5 MB'
	assert app.format_size('en', 512) == '512 B'
	d := time.new(year: 2026, month: 11, day: 7, hour: 15, minute: 4)
	assert app.format_date('vi', d) == '07/11/2026'
	assert app.format_date('en', d) == 'Nov 7, 2026'
	assert app.format_time('vi', d) == '15:04'
	assert app.format_time('en', d) == '3:04 PM'
	assert app.format_relative('vi', d, d.add_seconds(300)) == '5 phút trước'
	assert app.format_relative('en', d, d.add_seconds(3600)) == '1 hour ago'
	assert app.format_relative('vi', d, d.add_days(1)) == 'hôm qua'
}
