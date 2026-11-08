// Overlay pages: menu, context menu, tooltip, popover, hover card, dialog, confirm, toast.
module pages

import quadra.core
import quadra.kit
import quadra.ui

fn demo_menu() []kit.MenuItem {
	return [
		kit.MenuItem{
			label:     'Sao chép'
			glyph:     kit.icon_copy
			shortcut:  'Ctrl-C'
			on_select: fn (mut cx core.Cx) {
				kit.push_toast(mut cx, .success, 'Đã sao chép')
			}
		},
		kit.MenuItem{
			label:    'Đổi tên'
			glyph:    kit.icon_pencil
			shortcut: 'F2'
		},
		kit.MenuItem{
			label:    'Không khả dụng'
			disabled: true
		},
		kit.separator(),
		kit.MenuItem{
			label:  'Xoá'
			glyph:  kit.icon_trash_2
			danger: true
		},
	]
}

fn menu_page(_ &kit.Theme) core.Element {
	return page('Menu',
		'Actions in a popup, with icons and shortcuts. Arrow keys move, Enter picks, Escape closes.', [
		demo('Menu button', row([kit.menu_button('mn-file', 'Tệp', demo_menu()).icon(kit.icon_file),
			kit.menu_button('mn-more', 'Actions', demo_menu()).outline()]),
			"kit.menu_button('file', 'File', items)"),
	])
}

// editor_menu is an editor's context menu: groups split by separators, chords as
// shortcuts, entries that do not apply here disabled.
fn editor_menu() []kit.MenuItem {
	mut items := []kit.MenuItem{}
	for e in [['Go to Definition', 'f12'], ['Go to Implementation', 'ctrl-f12'],
		['Find All References', 'alt-shift-f12'], ['-'], ['Rename Symbol', 'f2'],
		['Format Selections', 'ctrl-k ctrl-f'], ['Show Code Actions', 'ctrl-.'],
		['-'], ['Cut', 'ctrl-x'], ['Copy', 'ctrl-c'], ['Paste', 'ctrl-v'],
		['-'], ['Open in Terminal', ''], ['Copy Permalink to Line', 'x']] {
		items << if e[0] == '-' {
			kit.separator()
		} else {
			kit.MenuItem{
				label:    e[0]
				shortcut: if e[1] == 'x' { '' } else { e[1] }
				disabled: e[1] == 'x'
			}
		}
	}
	return items
}

fn context_menu_page(t &kit.Theme) core.Element {
	return page('Context menu',
		'The same menu, opened by a right click or the Menu key on the target.', [
		demo('Target', kit.context_menu(ui.div().w(320).h(96).center().border(1).border_color(t.p.line_strong).child(hint(t,
			'Right-click here')), editor_menu()), 'kit.context_menu(target, items)'),
	])
}

fn tooltip_page(_ &kit.Theme) core.Element {
	return page('Tooltip',
		'A short label after a 500 ms hover or on keyboard focus. Never put actions inside.', [
		demo('Hover', row([kit.button('tt-btn').label('Hover me').tooltip('Tooltips appear after 500 ms'),
			kit.icon_button('tt-icon', kit.icon_refresh_cw).tooltip_keys('Refresh', 'f5')]),
			".tooltip_keys('Refresh', 'f5')"),
		demo('With detail', row([kit.tooltip(kit.button('tt-meta').label('Deploy'),
			'Deploy to staging').keys('ctrl-shift-d').meta('Builds the current branch first')]),
			"kit.tooltip(el, 'Deploy to staging').keys('ctrl-shift-d').meta('...')"),
	])
}

fn dialog_page(_ &kit.Theme) core.Element {
	return page('Dialog',
		'A modal task hanging near the top of the window. Escape, a click outside and the close button dismiss it.', [
		demo('Modal', kit.button('dl-open').label('Scale deployment…').primary().on_click(core.handler(fn (_ &core.ClickEvent, mut cx core.Cx) {
			kit.open_modal(mut cx, kit.ModalOptions{
				title: 'Scale deployment'
				body:  fn () core.Element {
					return ui.text('api will be scaled from 3 to 5 replicas.')
				}
			})
		})),
			"kit.open_modal(mut cx, kit.ModalOptions{ title: '...', body: fn () core.Element { ... } })"),
	])
}

pub struct ConfirmPage {
mut:
	answer string = '—'
}

@[markused]
pub fn (mut p ConfirmPage) render(mut cx core.ViewCx[ConfirmPage]) core.Element {
	t := kit.theme(cx)
	ask := cx.listener(fn (mut p ConfirmPage, _ &core.ClickEvent, mut cx core.ViewCx[ConfirmPage]) {
		kit.confirm(mut cx.Cx, 'Delete pod?', 'api-7d9f8c6b5-x2k4p will be terminated.', 'Delete', cx.listener(fn (mut p ConfirmPage, yes &bool, mut _ core.ViewCx[ConfirmPage]) {
			p.answer = if *yes { 'deleted' } else { 'cancelled' }
		}))
	})
	return page('Confirm', 'A yes-or-no question before a destructive action.', [
		demo('Delete', row([kit.button('cf-open').label('Delete pod').danger().on_click(ask),
			hint(t, 'Answer: ${p.answer}')]),
			"kit.confirm(mut cx, 'Delete pod?', message, 'Delete', cx.listener(...))"),
	])
}

fn toast_page(_ &kit.Theme) core.Element {
	return page('Toast',
		'A brief notice at the bottom center that expires after 10 s; notifications stay at the bottom right until closed.', [
		demo('Kinds', row([
			kit.button('to-info').label('Info').on_click(core.handler(fn (_ &core.ClickEvent, mut cx core.Cx) {
				kit.push_toast(mut cx, .info, 'Port-forward started on :8080')
			})),
			kit.button('to-ok').label('Success').on_click(core.handler(fn (_ &core.ClickEvent, mut cx core.Cx) {
				kit.push_toast(mut cx, .success, 'Deployment rolled out')
			})),
			kit.button('to-warn').label('Warning').on_click(core.handler(fn (_ &core.ClickEvent, mut cx core.Cx) {
				kit.push_toast(mut cx, .warning, 'Node pool-a-3 is under memory pressure')
			})),
			kit.button('to-err').label('Error').on_click(core.handler(fn (_ &core.ClickEvent, mut cx core.Cx) {
				kit.push_toast(mut cx, .error, 'Image pull failed: nginx:1.99')
			})),
		]), "kit.push_toast(mut cx, .success, 'Deployment rolled out')"),
		demo('Notification', row([
			kit.button('to-note').label('Notify').on_click(core.handler(fn (_ &core.ClickEvent, mut cx core.Cx) {
				kit.notify(mut cx, .info, 'Language server installed',
					'v-analyzer 0.0.6 is ready. It starts with the next file you open.',
					actions: [
						kit.NoticeAction{
							label: 'Release Notes'
						},
						kit.NoticeAction{
							label:   'Open a File'
							primary: true
						},
					]
				)
				kit.notify(mut cx, .success, 'Saved', 'This one closes by itself after 5 s.',
					timeout_ms: 5000
				)
			})),
		]), "kit.notify(mut cx, .info, 'Language server installed', 'v-analyzer 0.0.6 is ready.')"),
	])
}

fn popover_page(_ &kit.Theme) core.Element {
	return page('Popover',
		'A framed panel next to its trigger for settings or details: a click opens it, a click outside or Escape closes it.', [
		demo('Filter', kit.popover_button('pv-open', 'Columns', fn () core.Element {
			return ui.div().flex_col().gap(8).w(220).child(ui.text('Visible columns').weight(600)).child(kit.checkbox('pv-name',
				true).label('Name')).child(kit.checkbox('pv-age', true).label('Age')).child(kit.checkbox('pv-node',
				false).label('Node'))
		}).icon(kit.icon_sliders_horizontal),
			"kit.popover_button('columns', 'Columns', fn () core.Element { ... })"),
	])
}

fn hover_card_page(t &kit.Theme) core.Element {
	return page('Hover card',
		'Details about a link or a name after a 400 ms hover. It stays open while the pointer is over it, so its links can be clicked.', [
		demo('Pod', kit.hover_card(ui.div().test_id('hc-trigger').child(ui.text('api-7d9f8c6b5-x2k9q').color(t.p.accent)), fn () core.Element {
			return ui.div().flex_col().gap(4).w(240).child(ui.text('api-7d9f8c6b5-x2k9q').weight(600)).child(ui.text('Running on node-2 · 3 restarts')).child(kit.button('hc-logs').label('Open logs').size(.sm))
		}), 'kit.hover_card(trigger, fn () core.Element { ... })'),
	])
}
