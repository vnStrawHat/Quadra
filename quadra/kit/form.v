// Forms: labelled fields with help and error text, validation rules, and confirm().
module kit

import quadra.core
import quadra.ui

pub type Rule = fn (value string) ?string // returns an error message, or none when valid

pub fn required(msg string) Rule {
	return fn [msg] (v string) ?string {
		return if v.trim_space() == '' { msg } else { none }
	}
}

pub fn min_len(n int, msg string) Rule {
	return fn [n, msg] (v string) ?string {
		return if v.runes().len < n { msg } else { none }
	}
}

pub fn max_len(n int, msg string) Rule {
	return fn [n, msg] (v string) ?string {
		return if v.runes().len > n { msg } else { none }
	}
}

pub fn email(msg string) Rule {
	return fn [msg] (v string) ?string {
		at := v.index('@') or { return msg }
		return if at > 0 && v[at + 1..].contains('.') && !v.contains(' ') { none } else { msg }
	}
}

// validate runs rules in order and returns the first error.
pub fn validate(value string, rules []Rule) ?string {
	for r in rules {
		if msg := r(value) {
			return msg
		}
	}
	return none
}

// field wraps a control with a label, a required marker, help and error text.
pub fn field(label string, control core.Element, help string, error string, required_mark bool) &Styled {
	return themed_with(FieldArgs{label, control, help, error, required_mark}, fn (a FieldArgs, t &Theme) core.Element {
		control, help, error, required_mark := a.control, a.help, a.error, a.required_mark
		p := t.p
		// Label, control, then the error or the help, 12 px and 4 apart (specs/15).
		mut head := ui.div().flex_row().gap(4).child(ui.text(a.label).size(t.ui_text(.sm)))
		if required_mark {
			head.child(ui.text('*').size(t.ui_text(.sm)).color(p.error))
		}
		mut col := ui.div().flex_col().gap(4).child(head).child(control)
		if error != '' {
			col.child(ui.div().flex_row().items_center().gap(4).role(.alert).label(error).child(icon(icon_circle_alert).size(12).color(p.error)).child(ui.text(error).size(t.ui_text(.sm)).color(p.error)))
		} else if help != '' {
			col.child(ui.text(help).size(t.ui_text(.sm)).color(p.text_2))
		}
		return col
	})
}

// confirm asks a yes/no question in a modal and reports the answer to the listener.
pub fn confirm(mut cx core.Cx, title string, message string, yes string, l core.Listener[bool]) int {
	answer := l.any
	cancel := strings_of(cx.app).cancel
	mut holder := &ModalId{}
	card := theme_of(cx.app).ctl.card_dialogs
	gap := if card { f32(8) } else { 4 }
	id := open_modal(mut cx, ModalOptions{
		title:       title
		width:       if card { 448 } else { 440 }
		description: message
		alert:       true
		footer:      fn [answer, yes, cancel, gap, mut holder] () core.Element {
			return ui.div().flex_row().gap(gap).child(button('confirm-no').label(cancel).on_click(core.handler_with(Answer{answer, holder, false},
				answer_click))).child(button('confirm-yes').label(yes).primary().on_click(core.handler_with(Answer{answer, holder, true},
				answer_click)))
		}
	})
	holder.id = id
	return id
}

// ModalId lets a modal's own buttons know its id (closures capture by value).
@[heap]
struct ModalId {
mut:
	id int
}

struct FieldArgs {
	label         string
	control       core.Element
	help          string
	error         string
	required_mark bool
}

// Answer is a confirm button's data: the listener, the modal and the answer it gives.
struct Answer {
	answer core.AnyListener
	modal  &ModalId = unsafe { nil }
	value  bool
}

fn answer_click(a Answer, _ &core.ClickEvent, mut cx core.Cx) {
	close_modal(mut cx, a.modal.id)
	v := a.value
	cx.app.invoke(a.answer, &v, cx.win, true)
}
