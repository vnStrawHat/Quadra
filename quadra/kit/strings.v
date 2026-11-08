// The few words kit components show by themselves. English by default; an app in another
// language installs its own with set_strings (Notes does it when the locale changes).
module kit

import quadra.core

pub struct Strings {
pub:
	cancel              string   = 'Cancel'
	select_placeholder  string   = 'Select…'
	commands            string   = 'Commands'
	command_placeholder string   = 'Type a command…'
	no_commands         string   = 'No matching commands'
	app_menu            string   = 'Application menu'
	search_settings     string   = 'Search settings…'
	weekdays            []string = ['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su']
	months              []string = ['January', 'February', 'March', 'April', 'May', 'June', 'July',
		'August', 'September', 'October', 'November', 'December']
	month_title         string = '{month} {year}'
}

pub fn strings_vi() Strings {
	return Strings{
		cancel:              'Huỷ'
		select_placeholder:  'Chọn…'
		commands:            'Lệnh'
		command_placeholder: 'Nhập lệnh…'
		no_commands:         'Không tìm thấy lệnh'
		app_menu:            'Menu ứng dụng'
		search_settings:     'Tìm cài đặt…'
		weekdays:            ['T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'CN']
		months:              []string{len: 12, init: 'Tháng ${index + 1}'}
		month_title:         '{month}, {year}'
	}
}

pub fn set_strings(mut a core.App, s Strings) {
	mut cx := core.Cx{
		app: a
	}
	cx.set_global(s)
	for mut w in a.windows {
		w.invalidate()
	}
}

// strings_of returns the installed strings (layout time: no dependency is recorded).
pub fn strings_of(a &core.App) Strings {
	cx := core.Cx{
		app: unsafe { a }
	}
	s := cx.peek_global[Strings]() or { return Strings{} }
	return *s
}

fn (s &Strings) month_year(month int, year int) string {
	return s.month_title.replace('{month}', s.months[month - 1]).replace('{year}', year.str())
}
