// The dialogs of the browser: questions, name forms, properties.
module sftpui

import ssh
import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.prim
import quadra.ui

@[heap]
struct ModalRef {
mut:
	id int
}

struct Answer {
	l     core.AnyListener
	modal &ModalRef = unsafe { nil }
	yes   bool
}

fn answer_click(a Answer, _ &core.ClickEvent, mut cx core.Cx) {
	kit.close_modal(mut cx, a.modal.id)
	v := a.yes
	cx.app.invoke(a.l, &v, cx.win, true)
}

// ask opens a question with two buttons; the listener gets the answer. Escape answers nothing.
fn (p &SftpPanel) ask(mut cx core.ViewCx[SftpPanel], title string, body string, yes string, no string, danger bool, l core.Listener[bool]) {
	p.bind(mut cx)
	any := l.any
	mut m := &ModalRef{}
	m.id = kit.open_modal(mut cx.Cx, kit.ModalOptions{
		title:       title
		width:       440
		body_with:   kit.build_with(body, fn (body string, _ &kit.Theme) core.Element {
			return ui.div().w_full().child(ui.text(body))
		})
		footer_with: kit.build_with(AskArgs{any, m, yes, no, danger}, ask_footer)
	})
}

struct AskArgs {
	any    core.AnyListener
	m      &ModalRef = unsafe { nil }
	yes    string
	no     string
	danger bool
}

fn ask_footer(a AskArgs, _ &kit.Theme) core.Element {
	mut y := kit.button('ask-yes').label(a.yes).on_click(core.handler_with(Answer{a.any, a.m, true},
		answer_click))
	if a.danger {
		y.danger()
	} else {
		y.primary()
	}
	return ui.div().flex_row().gap(4).child(kit.button('ask-no').label(a.no).on_click(core.handler_with(Answer{a.any, a.m, false},
		answer_click))).child(y)
}

// open_form asks for a name: label, input, a required mark. Submit (button or Enter) calls
// on_submit with the text; the owner closes the modal when the name was accepted.
fn open_form(mut cx core.Cx, title string, label string, value string, placeholder string, confirm string, on_submit core.Listener[string]) &ModalRef {
	field := input.new_input(mut cx, id: 'sftp-form-input', value: value, placeholder: placeholder)
	input.on_submit(mut cx, field, on_submit)
	submit := on_submit.any
	mut m := &ModalRef{}
	args := FormArgs{field, label, m, submit, confirm}
	m.id = kit.open_modal(mut cx, kit.ModalOptions{
		title:       title
		width:       440
		body_with:   kit.build_with(args, fn (a FormArgs, _ &kit.Theme) core.Element {
			return kit.field(a.label, core.view_element(a.field.any()), '', '', true)
		})
		footer_with: kit.build_with(args, fn (a FormArgs, _ &kit.Theme) core.Element {
			cancel := kit.button('form-cancel').label('Cancel').on_click(core.handler_with(a.m.id,
				close_on_click))
			return ui.div().flex_row().gap(4).child(cancel).child(kit.button('form-ok').label(a.confirm).primary().on_click(core.handler_with(FormSubmit{a.submit, a.field},
				form_submit)))
		})
	})
	cx.peek(field.model()).focus.focus(mut cx)
	return m
}

struct FormArgs {
	field   core.View[input.TextInput]
	label   string
	m       &ModalRef = unsafe { nil }
	submit  core.AnyListener
	confirm string
}

fn close_on_click(id int, _ &core.ClickEvent, mut cx core.Cx) {
	kit.close_modal(mut cx, id)
}

struct FormSubmit {
	l     core.AnyListener
	field core.View[input.TextInput]
}

fn form_submit(f FormSubmit, _ &core.ClickEvent, mut cx core.Cx) {
	v := cx.peek(f.field.model()).value()
	cx.app.invoke(f.l, &v, cx.win, true)
}

fn close_form(mut cx core.ViewCx[SftpPanel], m &ModalRef) {
	if m != unsafe { nil } {
		kit.close_modal(mut cx.Cx, m.id)
	}
}

fn prop_row(label string, value string, muted prim.Color) core.Element {
	return ui.div().flex_row().gap(8).py(4).w_full().child(ui.div().w(100).shrink_0().text_color(muted).child(ui.text(label))).child(ui.div().grow().min_w(0).overflow_hidden().child(ui.text(value).truncate()))
}

// show_properties opens the details of a stat result.
fn show_properties(mut cx core.ViewCx[SftpPanel], e ssh.FileEntry) {
	th := kit.theme(cx)
	_ = th
	muted := th.p.text_2
	kind := if e.is_dir { 'Folder' } else { 'File' }
	suffix := if e.is_symlink { ' (symlink)' } else { '' }
	size := if e.is_dir { '-' } else { '${format_size(e.size)} (${e.size} bytes)' }
	rows := [['Name:', e.name], ['Type:', kind + suffix], ['Size:', size],
		['Modified:', format_date(e.modified)], ['Accessed:', format_date(e.accessed)],
		['Permissions:', format_permissions(e.permissions)], ['Owner:', format_owner(e.owner, e.uid)],
		['Group:', format_owner(e.group, e.gid)], ['Path:', e.path]]
	mut m := &ModalRef{}
	m.id = kit.open_modal(mut cx.Cx, kit.ModalOptions{
		title:       'Properties'
		width:       480
		body_with:   kit.build_with(PropRows{rows, muted}, fn (a PropRows, _ &kit.Theme) core.Element {
			mut col := ui.div().flex_col().w_full()
			for r in a.rows {
				col.child(prop_row(r[0], r[1], a.muted))
			}
			return col
		})
		footer_with: kit.build_with(ModalBox{m}, fn (b ModalBox, _ &kit.Theme) core.Element {
			return kit.button('props-close').label('Close').primary().on_click(core.handler_with(b.m.id,
				close_on_click))
		})
	})
}

struct PropRows {
	rows  [][]string
	muted prim.Color
}

struct ModalBox {
	m &ModalRef = unsafe { nil }
}
