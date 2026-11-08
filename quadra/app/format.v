// Locale formatting for numbers, dates, times, relative times and file sizes (vi, en).
module app

import math
import time

struct Seps {
	group   string
	decimal string
}

fn seps(locale string) Seps {
	return match locale.all_before('-') {
		'vi', 'de', 'id' { Seps{'.', ','} }
		else { Seps{',', '.'} }
	}
}

pub fn format_int(locale string, n i64) string {
	return format_number(locale, f64(n), 0)
}

// format_number groups thousands and rounds to `decimals` places: vi 1.234,5 / en 1,234.5.
pub fn format_number(locale string, v f64, decimals int) string {
	sp := seps(locale)
	neg := v < 0
	scale := math.pow(10, decimals)
	r := i64(math.round(math.abs(v) * scale))
	whole := (r / i64(scale)).str()
	mut grouped := []u8{}
	for i, c in whole {
		if i > 0 && (whole.len - i) % 3 == 0 {
			grouped << sp.group.bytes()
		}
		grouped << c
	}
	mut out := (if neg { '-' } else { '' }) + grouped.bytestr()
	if decimals > 0 {
		frac := (r % i64(scale)).str()
		out += sp.decimal + '0'.repeat(decimals - frac.len) + frac
	}
	return out
}

const en_months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov',
	'Dec']

// format_date: vi 07/11/2026, en Nov 7, 2026.
pub fn format_date(locale string, t time.Time) string {
	if locale.starts_with('vi') {
		return '${t.day:02}/${t.month:02}/${t.year}'
	}
	return '${en_months[t.month - 1]} ${t.day}, ${t.year}'
}

// format_time: vi 15:04, en 3:04 PM.
pub fn format_time(locale string, t time.Time) string {
	if locale.starts_with('vi') {
		return '${t.hour:02}:${t.minute:02}'
	}
	h := if t.hour % 12 == 0 { 12 } else { t.hour % 12 }
	return '${h}:${t.minute:02} ${if t.hour < 12 {
		'AM'
	} else {
		'PM'
	}}'
}

// format_size: 1,5 MB (vi) / 1.5 MB (en); binary units.
pub fn format_size(locale string, bytes i64) string {
	units := ['B', 'KB', 'MB', 'GB', 'TB']
	mut v := f64(bytes)
	mut u := 0
	for v >= 1024 && u < units.len - 1 {
		v /= 1024
		u++
	}
	decimals := if u == 0 || v >= 100 { 0 } else { 1 }
	return '${format_number(locale, v, decimals)} ${units[u]}'
}

// format_relative describes `then` relative to `now`: "5 phút trước" / "5 minutes ago".
pub fn format_relative(locale string, then time.Time, now time.Time) string {
	secs := now.unix() - then.unix()
	vi := locale.starts_with('vi')
	if secs < 45 {
		return if vi { 'vừa xong' } else { 'just now' }
	}
	mins := (secs + 30) / 60
	if mins < 60 {
		return if vi { '${mins} phút trước' } else { plural_en(mins, 'minute') + ' ago' }
	}
	hours := (mins + 30) / 60
	if hours < 24 {
		return if vi { '${hours} giờ trước' } else { plural_en(hours, 'hour') + ' ago' }
	}
	days := (hours + 12) / 24
	if days == 1 {
		return if vi { 'hôm qua' } else { 'yesterday' }
	}
	if days < 7 {
		return if vi { '${days} ngày trước' } else { plural_en(days, 'day') + ' ago' }
	}
	return format_date(locale, then)
}

fn plural_en(n i64, unit string) string {
	return if n == 1 { '1 ${unit}' } else { '${n} ${unit}s' }
}
