// Drawer, alert, empty state and description list pages.
module pages

import quadra.core
import quadra.kit
import quadra.ui

fn drawer_page(_ &kit.Theme) core.Element {
	open := fn (side kit.ModalSide, _ &core.ClickEvent, mut cx core.Cx) {
		kit.open_modal(mut cx, kit.ModalOptions{
			side:  side
			title: 'Pod api-7d9f8c'
			width: if side == .bottom { f32(240) } else { 360 }
			body:  fn () core.Element {
				return kit.description_list([
					kit.Description{'Status', kit.badge('Running', .success)},
					kit.Description{'Node', ui.text('pool-a-3')},
					kit.Description{'Restarts', ui.text('2')},
				])
			}
		})
	}
	return page('Drawer',
		'A panel sliding over the window from an edge, for details and settings. Escape or a click outside closes it.', [
		demo('Sides', row([
			kit.button('drawer-right').label('Right').primary().on_click(core.handler_with(kit.ModalSide.right,
				open)),
			kit.button('drawer-left').label('Left').on_click(core.handler_with(kit.ModalSide.left,
				open)),
			kit.button('drawer-bottom').label('Bottom').on_click(core.handler_with(kit.ModalSide.bottom,
				open)),
		]),
			"kit.open_modal(mut cx, kit.ModalOptions{ side: .right, title: '...', width: 360, body: ... })"),
	])
}

fn alert_page(_ &kit.Theme) core.Element {
	return page('Alert',
		'A message in the page flow, tinted by its tone. Errors and warnings are announced to screen readers.', [
		demo('Tones', ui.div().w(520).child(col([
			kit.alert(.info, 'Update available', 'Version 0.4 adds docking and Markdown help.'),
			kit.alert(.success, 'Deployed', 'api is running 5 replicas.'),
			kit.alert(.warning, 'High memory', 'node pool-a-3 is at 91%.'),
			kit.alert(.danger, 'Probe failed', '/healthz timed out 3 times.'),
		])), "kit.alert(.warning, 'High memory', 'node pool-a-3 is at 91%.')"),
	])
}

fn empty_state_page(t &kit.Theme) core.Element {
	return page('Empty state',
		'What a list, table or search shows when it has nothing: say why and offer the next step.', [
		demo('No results', framed(t, kit.empty_state(kit.icon_search, 'No pods match',
			'Try another name or clear the filter.', kit.button('es-clear').label('Clear filter'))).w(420),
			"kit.empty_state(kit.icon_search, 'No pods match', 'Try another name…', action)"),
	])
}

fn description_list_page(t &kit.Theme) core.Element {
	return page('Description list',
		'Terms and their values in two aligned columns: details panels and property sheets.', [
		demo('Pod', kit.description_list([
			kit.Description{'Name', ui.text('api-7d9f8c')},
			kit.Description{'Namespace', ui.text('production')},
			kit.Description{'Status', kit.badge('Running', .success)},
			kit.Description{'Image', ui.text('registry.example.com/api:1.8.2').family(t.mono_family)},
		]), "kit.description_list([kit.Description{'Name', ui.text('api')}, ...])"),
	])
}
