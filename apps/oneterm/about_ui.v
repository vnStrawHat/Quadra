// The About dialog and the Security, Network and About settings pages (specs/22, Services).
module main

import quadra.core
import quadra.kit
import quadra.ui

const repo_url = 'https://github.com/vnStrawHat/OneTerm'
const elevated_updates_text = 'Updates are checked and installed from the normal OneTerm window.'

// AboutView is the live body of the About dialog.
pub struct AboutView {
pub mut:
	one      core.Model[OneTerm]
	observed bool
}

fn open_about(one core.Model[OneTerm], mut cx core.Cx) {
	view := cx.new_view(AboutView{
		one: one
	})
	mut holder := &ModalRef{}
	body := fn [view] () core.Element {
		return core.view_element(view.any())
	}
	foot := fn [one, holder] () core.Element {
		mut row := ui.div().flex_row().justify_end().gap(8)
		if !is_restricted() {
			row.child(kit.button('about-check-update').label('Check for Updates').ghost().on_click(core.handler_with(one,
				about_check)))
		}
		row.child(kit.button('about-close').label('Close').on_click(core.handler_with(ModalClose{holder},
			about_close)))
		return row
	}
	holder.id = kit.open_modal(mut cx, kit.ModalOptions{
		title:  'About OneTerm'
		width:  520
		body:   body
		footer: foot
	})
}

fn about_check(one core.Model[OneTerm], _ &core.ClickEvent, mut cx core.Cx) {
	cx.update_view(core.View[OneTerm]{one.id, one.gen}, fn (mut o OneTerm, mut cx core.ViewCx[OneTerm]) {
		o.check_for_updates(mut cx)
	})
}

struct ModalClose {
	holder &ModalRef = unsafe { nil }
}

fn about_close(c ModalClose, _ &core.ClickEvent, mut cx core.Cx) {
	kit.close_modal(mut cx, c.holder.id)
}

// repo_link is the repository URL, underlined in the link colour; a click opens it.
fn repo_link(t &kit.Theme) core.Element {
	link := ui.rich_text([ui.span(repo_url).link(repo_url).color(t.p.accent_text)]).size(t.font_size(.sm)).on_link(core.handler(fn (url &string, mut cx core.Cx) {
		cx.app.plat.open_external(*url)
	}))
	return ui.div().py(t.sp(2)).child(link)
}

// app_identity is the About dialog's and the About page's header, the same in both.
fn app_identity(t &kit.Theme) core.Element {
	return ui.div().flex_col().items_center().justify_center().gap(12).w_full().child(kit.icon(icon_terminal_logo).size(96).color(brand_accent)).child(ui.text('OneTerm').size(t.scaled(t.text_xl))).child(ui.text('Version ${version}').size(t.font_size(.sm)).color(t.p.text_2)).child(ui.text('A Terminal application for SSH / SFTP / Local Shell').size(t.font_size(.sm)).color(t.p.text_2))
}

@[markused]
pub fn (mut a AboutView) render(mut cx core.ViewCx[AboutView]) core.Element {
	t := kit.theme(cx)
	if !a.observed {
		core.observe[AboutView, OneTerm](mut cx, a.one, fn (mut _ AboutView, _ core.Model[OneTerm], mut cx core.ViewCx[AboutView]) {
			cx.notify()
		})
		a.observed = true
	}
	o := cx.peek(a.one)
	mut col := ui.div().flex_col().gap(t.sp(20)).w_full()
	col.child(app_identity(t))
	// Links: the label left, the underlined URL right in the link colour.
	col.child(ui.div().test_id('about-repo-link').flex_row().items_center().justify_between().gap(t.sp(12)).w_full().child(ui.text('GitHub Repository').size(t.font_size(.sm))).child(repo_link(t)))
	if is_restricted() {
		col.child(ui.text(elevated_updates_text).size(t.font_size(.sm)).color(t.p.text_2))
		return col
	}
	mut updates :=
		ui.div().flex_col().gap(4).child(ui.text('Update Status').size(t.font_size(.sm))).child(ui.text(o.upd.status_text(version)).size(t.font_size(.sm)).color(t.p.text_2))
	col.child(updates)
	if o.upd.shows_install_button() {
		mut row := ui.div().flex_row().gap(8)
		row.child(kit.button('about-install-update').primary().label(o.upd.install_label()).disabled(!o.upd.can_install()).on_click(core.handler_with(a.one,
			about_install)))
		if o.upd.can_skip() {
			row.child(kit.button('about-skip-update').ghost().label('Skip This Version').on_click(core.handler_with(a.one,
				about_skip)))
		}
		col.child(row)
	}
	return col
}

fn about_install(one core.Model[OneTerm], _ &core.ClickEvent, mut cx core.Cx) {
	mut o := cx.peek(one)
	o.ask_install(mut cx, one)
}

fn about_skip(one core.Model[OneTerm], _ &core.ClickEvent, mut cx core.Cx) {
	cx.update_view(core.View[OneTerm]{one.id, one.gen}, fn (mut o OneTerm, mut cx core.ViewCx[OneTerm]) {
		o.skip_offered(mut cx)
	})
}

// open_dialog is the listener of each Security button.
fn (mut w SettingsWin) open_dialog(kind DialogKind, mut cx core.ViewCx[SettingsWin]) {
	open_vault_dialog(kind, w.one, mut cx.Cx)
}

fn (w &SettingsWin) dialog_btn(id string, label string, kind DialogKind, busy bool, mut cx core.ViewCx[SettingsWin]) &kit.Button {
	return kit.button(id).label(label).disabled(busy).on_click(core.listener_with[SettingsWin, DialogKind, core.ClickEvent](cx.entity,
		kind, fn (mut w SettingsWin, kind DialogKind, _ &core.ClickEvent, mut cx core.ViewCx[SettingsWin]) {
		w.open_dialog(kind, mut cx)
	}))
}

fn (mut w SettingsWin) security_groups(mut cx core.ViewCx[SettingsWin]) []kit.SettingsGroup {
	o := cx.peek(w.one)
	busy := o.vault.is_busy()
	lock_now := kit.button('sec-lock').label('Lock Now').outline().on_click(cx.listener(fn (mut w SettingsWin, _ &core.ClickEvent, mut cx core.ViewCx[SettingsWin]) {
		mut o := cx.peek(w.one)
		o.lock_screen_from_settings(mut cx.Cx)
		cx.touch(w.one)
		cx.notify()
	}))
	mut primary := core.Element(ui.div())
	if !o.vault.is_read_only() && o.vault.phase() == .uninitialised {
		primary = core.Element(w.dialog_btn('sec-setup', 'Set Up...', .set_up, busy, mut cx).primary())
	} else if o.vault.phase() == .locked {
		primary = core.Element(w.dialog_btn('sec-unlock', 'Unlock...', .unlock, busy, mut cx).primary())
	}
	mut rows := [kit.SettingsRow{'Status', vault_status_text(o), primary}]
	if o.vault.can_change() {
		rows << kit.SettingsRow{'Change', 'Saved passwords stay valid; only the master password changes.', w.dialog_btn('sec-change',
			'Change...', .change, busy, mut cx).outline()}
	}
	if o.vault.can_lock_screen() {
		rows << kit.SettingsRow{'Lock Now', 'Cover every OneTerm window until the master password is typed (also Lock OneTerm in the app menu, Ctrl+Shift+L by default).', lock_now}
	}
	if o.vault.can_reset() {
		rows << kit.SettingsRow{'Reset', 'Delete the master password and every saved password. Sessions are kept.', w.dialog_btn('sec-reset',
			'Reset...', .reset, busy, mut cx).danger()}
	}
	return [
		group('Master Password', rows),
		group('Screen Lock', [
			w.number_row('lock_mins', 'Lock After Idle (minutes)',
				'With a master password set up, OneTerm starts locked, and every OneTerm window is covered after this long without keyboard or mouse input (1 to 480 minutes). Terminal output does not count as input.', mut
				cx),
		]),
	]
}

fn (mut w SettingsWin) network_groups(mut cx core.ViewCx[SettingsWin]) []kit.SettingsGroup {
	o := cx.peek(w.one)
	t := kit.theme(cx)
	mut rows := [
		w.text_row('proxy', 'Proxy URL', 'Blank uses system proxy.', '', mut cx),
		w.switch_row('verify_tls', 'Verify Certificates',
			'Verify GitHub TLS certificates. Turn off only on a trusted network that intercepts TLS.', mut cx),
	]
	if !o.update.verify_certificates {
		rows << kit.SettingsRow{'Insecure: certificate verification is disabled', 'Anyone on the network path can serve a forged release list, archive, and matching checksum. Downloaded updates are not authenticated until this is re-enabled.', ui.text('Insecure').color(t.p.error)}
	}
	return [
		group_about('GitHub Connection',
			'How OneTerm reaches GitHub Releases to check for and download updates.', rows),
	]
}

fn (mut w SettingsWin) about_groups(mut cx core.ViewCx[SettingsWin]) []kit.SettingsGroup {
	o := cx.peek(w.one)
	t := kit.theme(cx)
	mut update_rows := []kit.SettingsRow{}
	if is_restricted() {
		update_rows << kit.SettingsRow{elevated_updates_text, '', ui.div()}
	} else {
		check := kit.button('about-check').label('Check Now').disabled(o.upd.is_busy()).on_click(cx.listener(fn (mut w SettingsWin, _ &core.ClickEvent, mut cx core.ViewCx[SettingsWin]) {
			cx.update_view(core.View[OneTerm]{w.one.id, w.one.gen}, fn (mut o OneTerm, mut cx core.ViewCx[OneTerm]) {
				o.check_for_updates(mut cx)
			})
		}))
		update_rows << w.switch_row('auto_check', 'Automatic Checks',
			'Check GitHub Releases at startup once the interval has elapsed.', mut cx)
		update_rows << w.number_row('check_h', 'Check Interval (hours)',
			'Minimum time between automatic checks.', mut cx)
		update_rows << w.choice_row('channel', 'Channel',
			'Preview also offers GitHub prereleases; drafts are never offered.', [
			'Stable',
			'Preview (includes prereleases)',
		], ['stable', 'preview'], mut cx)
		if o.update.skipped_version != '' {
			update_rows << kit.SettingsRow{'Skipped Version', 'Clear to be offered this version again.', ui.div().flex_row().items_center().gap(8).child(ui.text('OneTerm ${o.update.skipped_version} is skipped and will not be offered again.').color(t.p.text_2)).child(kit.button('update-clear-skipped-version').label('Clear').ghost().size(.sm).on_click(cx.listener(fn (mut w SettingsWin, _ &core.ClickEvent, mut cx core.ViewCx[SettingsWin]) {
				mut o := cx.peek(w.one)
				o.update.skipped_version = ''
				w.changed(mut cx)
			})))}
		}
		update_rows << kit.SettingsRow{'Check for Updates', 'Ask GitHub Releases now, whatever the interval says.', check}
		update_rows << kit.SettingsRow{'Update Status', '', ui.text(o.upd.status_text(version)).color(t.p.text_2)}
	}
	return [
		group('Application', [
			kit.SettingsRow{'', '', app_identity(t)},
		]),
		group('Links', [
			kit.SettingsRow{'GitHub Repository', '', repo_link(t)},
		]),
		group('Updates', update_rows),
	]
}
