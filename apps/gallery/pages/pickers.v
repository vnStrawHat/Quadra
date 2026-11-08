// Picker pages: select, date picker, form field with validation.
module pages

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui

pub struct SelectPage {
mut:
	ns    int = -1
	fruit int = 1
}

@[markused]
pub fn (mut p SelectPage) render(mut cx core.ViewCx[SelectPage]) core.Element {
	return page('Select',
		'Picks one value from a list in a popup. Escape or an outside click closes it.', [
		demo('With a placeholder', kit.select_box('sel-ns', ['default', 'kube-system', 'monitoring',
			'ingress-nginx'], p.ns).placeholder('Namespace').width(220).on_change(cx.listener(fn (mut p SelectPage, i &int, mut _ core.ViewCx[SelectPage]) {
			p.ns = *i
		})), "kit.select_box('ns', names, v.ns).placeholder('Namespace').on_change(...)"),
		demo('With a value', kit.select_box('sel-fruit', ['Táo', 'Cam', 'Xoài', 'Sầu riêng'],
			p.fruit).width(220).on_change(cx.listener(fn (mut p SelectPage, i &int, mut _ core.ViewCx[SelectPage]) {
			p.fruit = *i
		})), ''),
	])
}

pub struct DatePage {
mut:
	date kit.Date = kit.today()
	cal  core.View[kit.Calendar]
	day  kit.Date
	time kit.TimeOfDay = kit.TimeOfDay{9, 30}
}

@[markused]
pub fn (mut p DatePage) render(mut cx core.ViewCx[DatePage]) core.Element {
	t := kit.theme(cx)
	if p.cal.id == 0 {
		p.cal = kit.new_calendar(mut cx.Cx, 'dp-cal', p.date)
		kit.on_date(mut cx.Cx, p.cal, cx.listener(fn (mut p DatePage, d &kit.Date, mut _ core.ViewCx[DatePage]) {
			p.day = *d
		}))
	}
	return page('Date and time',
		'A date in a calendar popup, a calendar in the page, and a time of day typed or stepped with the arrow keys.', [
		demo('Basic', row([
			kit.date_picker('dp-date', p.date).on_change(cx.listener(fn (mut p DatePage, d &kit.Date, mut _ core.ViewCx[DatePage]) {
				p.date = *d
			})),
			mono(t, '${p.date}'),
		]), "kit.date_picker('since', v.since).on_change(...)"),
		demo('Calendar', row([
			cx.child_view(p.cal),
			mono(t, if p.day.year == 0 { '' } else { '${p.day}' }),
		]), "kit.new_calendar(mut cx, 'cal', today) + kit.on_date(...)"),
		demo('Time', row([
			kit.time_input('dp-time', p.time).minute_step(5).on_change(cx.listener(fn (mut p DatePage, v &kit.TimeOfDay, mut _ core.ViewCx[DatePage]) {
				p.time = *v
			})),
			mono(t, '${p.time}'),
		]), "kit.time_input('at', v.at).minute_step(5).on_change(...)"),
	])
}

pub struct FieldPage {
mut:
	email   core.View[input.TextInput]
	name    core.View[input.TextInput]
	agree   bool
	tried   bool
	email_v string
	name_v  string
}

@[markused]
pub fn (mut p FieldPage) render(mut cx core.ViewCx[FieldPage]) core.Element {
	if p.email.id == 0 {
		p.email = input.new_input(mut cx.Cx, id: 'f-email', placeholder: 'ban@vidu.vn')
		p.name = input.new_input(mut cx.Cx, id: 'f-name', placeholder: 'Nguyễn Văn An')
		input.on_change(mut cx.Cx, p.email, cx.listener(fn (mut p FieldPage, v &string, mut cx core.ViewCx[FieldPage]) {
			p.email_v = *v
			p.mark(mut cx)
		}))
		input.on_change(mut cx.Cx, p.name, cx.listener(fn (mut p FieldPage, v &string, mut cx core.ViewCx[FieldPage]) {
			p.name_v = *v
			p.mark(mut cx)
		}))
	}
	name_err := if p.tried { kit.validate(p.name_v, [
			kit.required('Vui lòng nhập họ tên'),
			kit.min_len(3, 'Tối thiểu 3 ký tự'),
		]) or { '' } } else { '' }
	email_err := if p.tried { kit.validate(p.email_v, [
			kit.required('Vui lòng nhập email'),
			kit.email('Email không hợp lệ'),
		]) or { '' } } else { '' }
	submit := cx.listener(fn (mut p FieldPage, _ &core.ClickEvent, mut cx core.ViewCx[FieldPage]) {
		p.tried = true
		p.mark(mut cx)
		ok := kit.validate(p.name_v, [kit.required(''), kit.min_len(3, '')]) == none
			&& kit.validate(p.email_v, [kit.email('')]) == none
		if ok {
			kit.push_toast(mut cx.Cx, .success, 'Đã gửi đăng ký cho ${p.name_v}')
		}
	})
	form := ui.div().flex_col().gap(16).w(380).child(kit.field('Họ tên', cx.child_view(p.name),
		'Như trên giấy tờ', name_err, true)).child(kit.field('Email', cx.child_view(p.email),
		'Chúng tôi không chia sẻ email của bạn', email_err, true)).child(kit.checkbox('f-agree',
		p.agree).label('Đồng ý điều khoản').on_change(cx.listener(fn (mut p FieldPage, v &bool, mut _ core.ViewCx[FieldPage]) {
		p.agree = *v
	}))).child(ui.div().flex_row().child(kit.button('f-submit').label('Đăng ký').primary().disabled(!p.agree).on_click(submit)))
	return page('Form field',
		'A label, a control, help text and an error. Rules validate on submit.', [
		demo('Sign-up', form,
			"kit.field('Email', control, help, kit.validate(v, [kit.required('...'), kit.email('...')]) or { '' }, true)"),
	])
}

// mark outlines the fields that fail their rules in the error color, once a submit was tried.
fn (p &FieldPage) mark(mut cx core.ViewCx[FieldPage]) {
	name_bad := p.tried && kit.validate(p.name_v, [kit.required(''), kit.min_len(3, '')]) != none
	email_bad := p.tried && kit.validate(p.email_v, [kit.required(''), kit.email('')]) != none
	input.set_invalid(mut cx.Cx, p.name, name_bad)
	input.set_invalid(mut cx.Cx, p.email, email_bad)
}
