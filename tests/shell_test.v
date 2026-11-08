// File dialogs and the shell through Cx, on the headless platform.
module main

import quadra.core
import quadra.platform
import quadra.testing
import quadra.ui

struct Picker {
mut:
	paths []string
}

@[markused]
pub fn (mut p Picker) render(mut cx core.ViewCx[Picker]) core.Element {
	return ui.div().size_full().child(ui.div().test_id('open').w(80).h(30).on_click(cx.listener(fn (mut p Picker, _ &core.ClickEvent, mut cx core.ViewCx[Picker]) {
		p.paths = cx.pick_files(platform.FileDialog{
			title:   'Open manifests'
			filters: [platform.FileFilter{'YAML', '*.yaml;*.yml'}]
		})
	}))).child(ui.div().test_id('docs').w(80).h(30).on_click(cx.listener(fn (mut p Picker, _ &core.ClickEvent, mut cx core.ViewCx[Picker]) {
		cx.open_external('https://example.com/docs')
		cx.reveal('C:/work/app.yaml')
	})))
}

fn test_pick_files_returns_the_choice_or_nothing() {
	mut h := testing.new_harness()!
	v := h.open(Picker{})!
	h.plat.picks << ['C:/a.yaml', 'C:/b.yml']
	h.click('open')!
	mut cx := core.Cx{
		app: h.app
	}
	assert cx.peek(v.model()).paths == ['C:/a.yaml', 'C:/b.yml']
	h.click('open')! // nothing queued: the user cancelled
	assert cx.peek(v.model()).paths == []
}

fn test_open_external_and_reveal_reach_the_shell() {
	mut h := testing.new_harness()!
	h.open(Picker{})!
	h.click('docs')!
	assert h.plat.opened == ['https://example.com/docs', 'reveal:C:/work/app.yaml']
}

struct Asker {
mut:
	answer platform.MessageResult = .ok
}

@[markused]
pub fn (mut a Asker) render(mut cx core.ViewCx[Asker]) core.Element {
	return ui.div().test_id('ask').w(80).h(30).on_click(cx.listener(fn (mut a Asker, _ &core.ClickEvent, mut cx core.ViewCx[Asker]) {
		a.answer = cx.message_box(platform.MessageBox{
			title:   'Delete pod'
			text:    'Delete api-7d9f8c?'
			kind:    .question
			buttons: .yes_no
		})
	}))
}

fn test_message_box_returns_the_answer() {
	mut h := testing.new_harness()!
	v := h.open(Asker{})!
	h.plat.answers << .yes
	h.click('ask')!
	mut cx := core.Cx{
		app: h.app
	}
	assert cx.peek(v.model()).answer == .yes
	assert h.plat.messages.len == 1
	assert h.plat.messages[0].text == 'Delete api-7d9f8c?'
	h.click('ask')! // nothing queued: closed
	assert cx.peek(v.model()).answer == .cancel
}

struct Tray {
mut:
	events []platform.TrayAction
}

@[markused]
pub fn (mut t Tray) render(mut cx core.ViewCx[Tray]) core.Element {
	return ui.div().test_id('arm').w(80).h(30).on_click(cx.listener(fn (mut t Tray, _ &core.ClickEvent, mut cx core.ViewCx[Tray]) {
		cx.set_tray('Cluster: 3 alerts')
		cx.show_notification('Pod restarted', 'api-7d9f8c restarted 3 times')
		cx.on_tray(cx.listener(fn (mut t Tray, e &platform.TrayEvent, mut _ core.ViewCx[Tray]) {
			t.events << e.action
		}))
	}))
}

fn test_tray_icon_notifications_and_clicks() {
	mut h := testing.new_harness()!
	v := h.open(Tray{})!
	h.click('arm')!
	assert h.plat.tray == 'Cluster: 3 alerts'
	assert h.plat.notices == ['Pod restarted: api-7d9f8c restarted 3 times']
	h.inject(platform.TrayEvent{.menu, 100, 200})
	h.inject(platform.TrayEvent{.click, 0, 0})
	mut cx := core.Cx{
		app: h.app
	}
	assert cx.peek(v.model()).events == [platform.TrayAction.menu, .click]
}
