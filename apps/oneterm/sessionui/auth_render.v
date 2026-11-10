// AuthForm's rendering: the method radio, the secret inputs and the saved-password rows.
module sessionui

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui
import store

const auth_methods = ['Password', 'Private Key', 'SSH Agent']

pub struct SecretField {
pub:
	id    int
	clear bool // true: the clear button, false: the eye toggle
}

// secret_input is an input with a trailing eye toggle (masked ones) and a clear button.
fn (a &AuthForm) secret_input(v core.View[input.TextInput], id int, masked bool, cx &core.ViewCx[AuthForm], t &kit.Theme) core.Element {
	w := cx.peek(v.model())
	mut buttons := ui.div().absolute().right(4).top(0).bottom(0).flex_row().items_center().gap(2)
	if masked {
		buttons.child(ui.div().test_id('${a.id}-mask-${id}').size(24).center().rounded(t.radius_md).cursor(.pointer).on_click(core.listener_with[AuthForm, SecretField, core.ClickEvent](cx.entity, SecretField{id, false},
			secret_click)).child(kit.icon(if w.password {
			kit.icon_eye
		} else {
			kit.icon_eye_off
		}).size(14).color(t.p.text_2)))
	}
	if w.value() != '' {
		buttons.child(ui.div().test_id('${a.id}-clear-${id}').size(24).center().rounded(t.radius_md).cursor(.pointer).on_click(core.listener_with[AuthForm, SecretField, core.ClickEvent](cx.entity, SecretField{id, true},
			secret_click)).child(kit.icon(kit.icon_circle_x).size(14).color(t.p.text_2)))
	}
	return ui.div().flex_col().w_full().child(core.view_element(v.any())).child(buttons)
}

fn secret_click(mut a AuthForm, f SecretField, _ &core.ClickEvent, mut cx core.ViewCx[AuthForm]) {
	v := match f.id {
		0 { a.password }
		1 { a.passphrase }
		else { a.key_path }
	}

	mut w := cx.peek(v.model())
	if f.clear {
		input.set_value(mut cx.Cx, v, '')
		a.edited(mut cx)
	} else {
		w.password = !w.password
		cx.touch(v.model())
		cx.notify()
	}
}

@[markused]
pub fn (mut a AuthForm) render(mut cx core.ViewCx[AuthForm]) core.Element {
	t := kit.theme(cx)
	mut col := ui.div().flex_col().gap(12).w_full()
	col.child(labelled('Authentication', true, kit.radio_group('${a.id}-method', auth_methods,
		int(a.method)).horizontal().on_change(cx.listener(fn (mut a AuthForm, i &int, mut cx core.ViewCx[AuthForm]) {
		a.method = store_method(*i)
		if a.method == .private_key && a.secrets {
			cx.peek(a.passphrase.model()).focus.focus(mut cx.Cx)
		}
		a.edited(mut cx)
	}))))
	if a.secrets && a.method == .password {
		col.child(a.password_block(t, mut cx))
	}
	if a.secrets && a.method == .agent {
		col.child(ui.div().child(ui.text('Keys come from the running SSH agent; nothing to enter.').size(t.font_size(.sm)).color(t.p.text_2)))
	}
	if a.method == .private_key {
		browse := kit.button('${a.id}-browse').label('Browse').outline().on_click(cx.listener(fn (mut a AuthForm, _ &core.ClickEvent, mut cx core.ViewCx[AuthForm]) {
			a.browse(mut cx)
		}))
		col.child(labelled('Private Key', true, ui.div().flex_row().gap(8).w_full().child(ui.div().grow().min_w(0).child(a.secret_input(a.key_path,
			2, false, cx, t))).child(browse)))
		if a.secrets {
			col.child(labelled('Passphrase', false, a.secret_input(a.passphrase, 1, true, cx, t)))
		}
	}
	if a.editor && a.method == .password {
		col.child(a.editor_block(t, mut cx))
	}
	return col
}

fn store_method(i int) store.AuthMethod {
	return match i {
		1 { .private_key }
		2 { .agent }
		else { .password }
	}
}

fn (mut a AuthForm) password_block(t &kit.Theme, mut cx core.ViewCx[AuthForm]) core.Element {
	if a.uses_saved() {
		return labelled('Password', false, ui.div().flex_row().items_center().gap(8).w_full().test_id('${a.id}-saved-row').track_focus(a.saved_focus).child(kit.icon(kit.icon_lock).size(14).color(t.p.text_2)).child(ui.div().grow().child(ui.text('Using saved password').size(t.font_size(.sm)))).child(kit.button('${a.id}-use-different').label('Use a different one').outline().size(.sm).on_click(cx.listener(fn (mut a AuthForm, _ &core.ClickEvent, mut cx core.ViewCx[AuthForm]) {
			a.using_saved = false
			cx.peek(a.password.model()).focus.focus(mut cx.Cx)
			a.edited(mut cx)
		}))))
	}
	mut col := ui.div().flex_col().gap(8).w_full()
	col.child(labelled('Password', false, a.secret_input(a.password, 0, true, cx, t)))
	if a.offers_save {
		col.child(a.save_checkbox(mut cx))
	}
	if a.note != '' {
		col.child(ui.text(a.note).size(t.font_size(.xs)).color(t.p.text_2))
	}
	return col
}

fn (mut a AuthForm) save_checkbox(mut cx core.ViewCx[AuthForm]) core.Element {
	label := if a.saved != none { update_password_label } else { save_password_label }
	return kit.checkbox('${a.id}-save', a.save_ticked).label(label).on_change(cx.listener(fn (mut a AuthForm, v &bool, mut cx core.ViewCx[AuthForm]) {
		a.save_ticked = *v
		a.edited(mut cx)
	}))
}

// editor_block is the session dialog's password row: saved (Replace, Remove) or new.
fn (mut a AuthForm) editor_block(t &kit.Theme, mut cx core.ViewCx[AuthForm]) core.Element {
	if !a.uses_saved() {
		mut col := ui.div().flex_col().gap(8).w_full()
		col.child(labelled('Password', false, a.secret_input(a.password, 0, true, cx, t)))
		col.child(a.save_checkbox(mut cx))
		if a.note != '' {
			col.child(ui.text(a.note).size(t.font_size(.xs)).color(t.p.text_2))
		}
		return col
	}
	return labelled('Password', false, ui.div().flex_row().items_center().gap(8).w_full().test_id('${a.id}-editor').child(kit.icon(kit.icon_lock).size(14).color(t.p.text_2)).child(ui.div().grow().child(ui.text('Saved password').size(t.font_size(.sm)))).child(kit.button('${a.id}-replace').label('Replace').outline().size(.sm).on_click(cx.listener(fn (mut a AuthForm, _ &core.ClickEvent, mut cx core.ViewCx[AuthForm]) {
		a.begin_replace()
		cx.peek(a.password.model()).focus.focus(mut cx.Cx)
		a.edited(mut cx)
	}))).child(kit.button('${a.id}-remove').label('Remove').outline().size(.sm).on_click(cx.listener(fn (mut a AuthForm, _ &core.ClickEvent, mut cx core.ViewCx[AuthForm]) {
		a.remove_saved()
		a.edited(mut cx)
	}))))
}
