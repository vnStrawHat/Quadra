// Action pages: button, icon button, toggle button.
module pages

import quadra.core
import quadra.kit
import quadra.ui

pub struct ButtonPage {
mut:
	clicks int
}

@[markused]
pub fn (mut p ButtonPage) render(mut cx core.ViewCx[ButtonPage]) core.Element {
	t := kit.theme(cx)
	count := cx.listener(fn (mut p ButtonPage, _ &core.ClickEvent, mut _ core.ViewCx[ButtonPage]) {
		p.clicks++
	})
	return page('Button',
		'Starts an action. Use one primary button per view; Enter and Space activate a focused button.', [
		demo('Variants', row([kit.button('b-primary').label('Deploy').primary().on_click(count),
			kit.button('b-secondary').label('Secondary').on_click(count),
			kit.button('b-outline').label('Outline').outline().on_click(count),
			kit.button('b-ghost').label('Ghost').ghost().on_click(count),
			kit.button('b-danger').label('Delete').danger().on_click(count),
			kit.button('b-link').label('Link').link().on_click(count),
			hint(t, 'clicked ${p.clicks}×')]),
			"kit.button('deploy').label('Deploy').primary().on_click(cx.listener(...))"),
		demo('Sizes', row([kit.button('s-xs').label('xs · 18').size(.xs),
			kit.button('s-sm').label('sm · 22').size(.sm), kit.button('s-md').label('md · 28'),
			kit.button('s-lg').label('lg · 32').size(.lg)]),
			"kit.button('id').label('...').size(.sm)"),
		demo('With icons', row([kit.button('i-pull').label('Pull').icon(kit.icon_download),
			kit.button('i-save').label('Save').icon(kit.icon_save).primary(),
			kit.button('i-restart').label('Restart').icon(kit.icon_refresh_cw).outline(),
			kit.button('i-logs').label('Logs').icon(kit.icon_external_link).ghost()]),
			"kit.button('pull').label('Pull').icon(kit.icon_download)"),
		demo('States', row([kit.button('st-dis').label('Disabled').disabled(true),
			kit.button('st-dis-p').label('Disabled').primary().disabled(true),
			kit.button('st-load').label('Applying').primary().loading(true)]),
			'.disabled(true)  .loading(true)'),
		demo('Dialog footer',
			ui.div().flex_row().gap(8).justify_end().child(kit.button('f-cancel').label('Cancel').ghost()).child(kit.button('f-diff').label('View diff').outline()).child(kit.button('f-apply').label('Apply').primary()),
			''),
	])
}

fn icon_button_page(t &kit.Theme) core.Element {
	return page('Icon button',
		'A square button with only an icon. Always give it a tooltip; it is also the accessible name.', [
		demo('Sizes', row([kit.icon_button('ib-xs', kit.icon_settings).size(.xs).tooltip('xs'),
			kit.icon_button('ib-sm', kit.icon_settings).size(.sm).tooltip('sm'),
			kit.icon_button('ib-md', kit.icon_settings).tooltip('md'),
			kit.icon_button('ib-lg', kit.icon_settings).size(.lg).tooltip('lg')]),
			"kit.icon_button('settings', kit.icon_settings).tooltip('Settings')"),
		demo('Panel actions', ui.div().flex_row().items_center().h(t.panel_header_h).px(8).bg(t.p.panel).border(1).border_color(t.p.line).child(kit.section_label('Pods',
			t)).child(ui.div().grow()).child(kit.icon_button('pa-filter', kit.icon_filter).size(.xs).tooltip('Filter')).child(kit.icon_button('pa-refresh',
			kit.icon_refresh_cw).size(.xs).tooltip('Refresh')).child(kit.icon_button('pa-more',
			kit.icon_ellipsis).size(.xs).tooltip('More')), ''),
		demo('Variants', row([kit.icon_button('iv-ghost', kit.icon_plus).tooltip('Ghost (default)'),
			kit.icon_button('iv-outline', kit.icon_plus).outline().tooltip('Outline'),
			kit.icon_button('iv-primary', kit.icon_plus).primary().tooltip('Primary'),
			kit.icon_button('iv-danger', kit.icon_trash_2).danger().tooltip('Delete'),
			kit.icon_button('iv-dis', kit.icon_x).disabled(true).tooltip('Disabled')]), ''),
	])
}

pub struct ToggleButtonPage {
mut:
	bold   bool = true
	italic bool
	follow bool = true
	wrap   bool
	kind   int
}

@[markused]
pub fn (mut p ToggleButtonPage) render(mut cx core.ViewCx[ToggleButtonPage]) core.Element {
	return page('Toggle button',
		'A button that stays pressed. The pressed state turns its label and icon the accent.', [
		demo('Formatting', row([
			kit.toggle_button('tg-bold', p.bold).icon(kit.icon_bold).tooltip('Bold').on_toggle(cx.listener(fn (mut p ToggleButtonPage, v &bool, mut _ core.ViewCx[ToggleButtonPage]) {
				p.bold = *v
			})),
			kit.toggle_button('tg-italic', p.italic).icon(kit.icon_italic).tooltip('Italic').on_toggle(cx.listener(fn (mut p ToggleButtonPage, v &bool, mut _ core.ViewCx[ToggleButtonPage]) {
				p.italic = *v
			})),
		]), "kit.toggle_button('bold', v.bold).icon(kit.icon_bold).on_toggle(...)"),
		demo('Log viewer options', row([
			kit.toggle_button('tg-follow', p.follow).label('Follow').icon(kit.icon_arrow_down).size(.sm).on_toggle(cx.listener(fn (mut p ToggleButtonPage, v &bool, mut _ core.ViewCx[ToggleButtonPage]) {
				p.follow = *v
			})),
			kit.toggle_button('tg-wrap', p.wrap).label('Wrap lines').size(.sm).on_toggle(cx.listener(fn (mut p ToggleButtonPage, v &bool, mut _ core.ViewCx[ToggleButtonPage]) {
				p.wrap = *v
			})),
		]), ''),
		demo('Filter chips', p.chips(mut cx),
			"kit.toggle_button('kind-0', on).label('All').size(.sm)"),
	])
}

// chips is a row of filter chips: one toggle per kind, the chosen one pressed.
fn (mut p ToggleButtonPage) chips(mut cx core.ViewCx[ToggleButtonPage]) core.Element {
	mut items := []core.Element{}
	for i, s in ['All', 'Languages', 'Themes', 'Icon Themes', 'Snippets'] {
		items << kit.toggle_button('kind-${i}', i == p.kind).label(s).size(.sm).on_toggle(core.listener_with[ToggleButtonPage, int, bool](cx.entity,
			i, fn (mut p ToggleButtonPage, i int, _ &bool, mut _ core.ViewCx[ToggleButtonPage]) {
			p.kind = i
		}))
	}
	return ui.div().flex_row().gap(4).children(items)
}
