// Text entry pages: text input, password, number, textarea, combobox.
module pages

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui

pub struct TextInputPage {
mut:
	name   core.View[input.TextInput]
	search core.View[input.TextInput]
	ro     core.View[input.TextInput]
	dis    core.View[input.TextInput]
	echo   string
}

@[markused]
pub fn (mut p TextInputPage) render(mut cx core.ViewCx[TextInputPage]) core.Element {
	if p.name.id == 0 {
		p.name = input.new_input(mut cx.Cx, id: 'in-name', placeholder: 'Họ và tên')
		p.search = input.new_input(mut cx.Cx,
			id:          'in-search'
			placeholder: 'Filter pods'
			prefix:      kit.icon_search
		)
		p.ro = input.new_input(mut cx.Cx, id: 'in-ro', value: 'sha256:4f8a9c01', readonly: true)
		p.dis = input.new_input(mut cx.Cx, id: 'in-dis', placeholder: 'Disabled', disabled: true)
		input.on_change(mut cx.Cx, p.name, cx.listener(fn (mut p TextInputPage, v &string, mut _ core.ViewCx[TextInputPage]) {
			p.echo = *v
		}))
	}
	t := kit.theme(cx)
	return page('Text input',
		'One line of text with grapheme-aware editing, undo/redo, clipboard and IME composition (Telex/VNI).', [
		demo('Basic', col([ui.div().w(320).child(cx.child_view(p.name)),
			hint(t, 'Bạn đã nhập: ${p.echo}')]),
			"input.new_input(mut cx.Cx, id: 'name', placeholder: 'Họ và tên')"),
		demo('With an icon', ui.div().w(320).child(cx.child_view(p.search)),
			'prefix: kit.icon_search'),
		demo('Read-only and disabled', row([ui.div().w(240).child(cx.child_view(p.ro)),
			ui.div().w(240).child(cx.child_view(p.dis))]), 'readonly: true  disabled: true'),
	])
}

pub struct PasswordPage {
mut:
	pass core.View[input.TextInput]
}

@[markused]
pub fn (mut p PasswordPage) render(mut cx core.ViewCx[PasswordPage]) core.Element {
	if p.pass.id == 0 {
		p.pass = input.new_input(mut cx.Cx,
			id:          'in-pass'
			placeholder: 'Mật khẩu'
			password:    true
			prefix:      kit.icon_lock
		)
	}
	return page('Password input', 'Masks what is typed and never copies it to the clipboard.', [
		demo('Basic', ui.div().w(320).child(cx.child_view(p.pass)),
			"input.new_input(mut cx.Cx, id: 'pass', password: true, prefix: kit.icon_lock)"),
	])
}

pub struct NumberPage {
mut:
	replicas core.View[input.NumberInput]
	cpu      core.View[input.NumberInput]
}

@[markused]
pub fn (mut p NumberPage) render(mut cx core.ViewCx[NumberPage]) core.Element {
	if p.replicas.id == 0 {
		p.replicas = input.new_number_input(mut cx.Cx, id: 'in-replicas', value: 3, min: 0, max: 99)
		p.cpu = input.new_number_input(mut cx.Cx,
			id:       'in-cpu'
			value:    0.5
			min:      0
			max:      16
			step:     0.25
			decimals: 2
		)
	}
	t := kit.theme(cx)
	return page('Number input',
		'A number with bounds and a step. Arrow keys and the stepper change it.', [
		demo('Integer', row([cx.child_view(p.replicas), hint(t, 'replicas, 0 to 99')]),
			"input.new_number_input(mut cx.Cx, id: 'replicas', value: 3, min: 0, max: 99)"),
		demo('Decimal', row([cx.child_view(p.cpu), hint(t, 'CPU cores, step 0.25')]),
			'step: 0.25, decimals: 2'),
	])
}

pub struct TextareaPage {
mut:
	notes core.View[input.TextInput]
}

@[markused]
pub fn (mut p TextareaPage) render(mut cx core.ViewCx[TextareaPage]) core.Element {
	if p.notes.id == 0 {
		p.notes = input.textarea(mut cx.Cx,
			id:          'in-notes'
			placeholder: 'Commit message (Enter for a new line, Ctrl-Enter to commit)'
			rows:        3
			max_rows:    8
		)
	}
	return page('Textarea',
		'Several lines of text. It grows with its content up to max_rows, then scrolls.', [
		demo('Auto-growing', ui.div().w(480).child(cx.child_view(p.notes)),
			'input.textarea(mut cx.Cx, rows: 3, max_rows: 8)'),
	])
}

pub struct ComboboxPage {
mut:
	city   core.View[input.Combobox]
	picked string
}

@[markused]
pub fn (mut p ComboboxPage) render(mut cx core.ViewCx[ComboboxPage]) core.Element {
	towns := ['Hà Nội', 'Hải Phòng', 'Đà Nẵng', 'Huế', 'Cần Thơ',
		'Thành phố Hồ Chí Minh', 'Nha Trang', 'Đà Lạt']
	if p.city.id == 0 {
		p.city = input.new_combobox(mut cx.Cx, 'in-city', towns,
			'Tìm thành phố (gõ không dấu cũng được)')
		input.on_pick(mut cx.Cx, p.city, cx.listener(fn [towns] (mut p ComboboxPage, i &int, mut _ core.ViewCx[ComboboxPage]) {
			p.picked = towns[*i]
		}))
	}
	t := kit.theme(cx)
	return page('Combobox',
		'A text input with a filtered list. Matching is fuzzy and ignores diacritics.', [
		demo('Search', col([ui.div().w(320).child(cx.child_view(p.city)),
			hint(t, 'Đã chọn: ${p.picked}')]),
			"input.new_combobox(mut cx.Cx, 'city', options, 'Search')"),
	])
}
