// Navigation pages: sidebar, toolbar, breadcrumb, menu bar, status bar, command palette.
module pages

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui

pub struct SidebarPage {
mut:
	selected string = 'pods'
}

@[markused]
pub fn (mut p SidebarPage) render(mut cx core.ViewCx[SidebarPage]) core.Element {
	t := kit.theme(cx)
	nav := kit.sidebar([
		kit.NavSection{'Workloads', [
			kit.NavItem{'pods', 'Pods', kit.icon_layers},
			kit.NavItem{'deploy', 'Deployments', kit.icon_refresh_cw},
			kit.NavItem{'jobs', 'Jobs', kit.icon_clock},
		]},
		kit.NavSection{'Config', [
			kit.NavItem{'maps', 'Config maps', kit.icon_file},
			kit.NavItem{'secrets', 'Secrets', kit.icon_lock},
		]},
	], p.selected).on_select(cx.listener(fn (mut p SidebarPage, id &string, mut _ core.ViewCx[SidebarPage]) {
		p.selected = *id
	}))
	return page('Sidebar',
		'Sections of places. The current place is two tones up, the hovered one a tone.', [
		demo('Cluster navigation', framed(t, ui.div().flex_row().h(260).child(nav).child(ui.div().grow().p(16).child(hint(t,
			'Selected: ${p.selected}')))), 'kit.sidebar(sections, v.current).on_select(...)'),
	])
}

fn toolbar_page(t &kit.Theme) core.Element {
	return page('Toolbar', 'A strip of related actions above the content it acts on.', [
		demo('Editor', framed(t, kit.toolbar([kit.icon_button('tb-bold', kit.icon_bold).tooltip('Bold'),
			kit.icon_button('tb-italic', kit.icon_italic).tooltip('Italic'),
			kit.vdivider(), kit.icon_button('tb-list', kit.icon_list).tooltip('List'),
			kit.icon_button('tb-img', kit.icon_image).tooltip('Image'),
			ui.div().grow(), kit.button('tb-save').label('Save').icon(kit.icon_save).size(.sm)])),
			'kit.toolbar([kit.icon_button(...), kit.vdivider(), ...])'),
	])
}

pub struct BreadcrumbPage {
mut:
	crumb int = 3
}

@[markused]
pub fn (mut p BreadcrumbPage) render(mut cx core.ViewCx[BreadcrumbPage]) core.Element {
	crumbs := ['cluster-prod', 'default', 'Deployments', 'api'][..p.crumb + 1]
	return page('Breadcrumb',
		'Where the view sits in a hierarchy. Every part but the last is a link.', [
		demo('Resource path', kit.breadcrumb(crumbs).on_select(cx.listener(fn (mut p BreadcrumbPage, i &int, mut _ core.ViewCx[BreadcrumbPage]) {
			p.crumb = *i
		})), "kit.breadcrumb(['cluster', 'default', 'api']).on_select(...)"),
	])
}

fn menubar_page(t &kit.Theme) core.Element {
	file := [kit.MenuItem{
		label:    'New file'
		shortcut: 'Ctrl-N'
	}, kit.MenuItem{
		label:    'Open…'
		shortcut: 'Ctrl-O'
	}, kit.separator(), kit.MenuItem{
		label: 'Exit'
	}]
	edit := [kit.MenuItem{
		label:    'Undo'
		shortcut: 'Ctrl-Z'
	}, kit.MenuItem{
		label:    'Redo'
		shortcut: 'Ctrl-Y'
	}]
	return page('Menu bar',
		'The application menus. Alt focuses the bar, arrow keys move between menus.', [
		demo('Application', framed(t, kit.menubar([kit.MenuDef{'File', file},
			kit.MenuDef{'Edit', edit}, kit.MenuDef{'View', edit}])),
			"kit.menubar([kit.MenuDef{'File', items}, ...])"),
	])
}

fn statusbar_page(t &kit.Theme) core.Element {
	bar := kit.statusbar([kit.status_label(.running, 'Connected'),
		kit.status_item(kit.icon_layers, 'cluster-prod'), kit.status_item('', 'default')], [
		kit.status_item('', 'Ln 12, Col 4'),
		kit.status_item('', 'UTF-8'),
		kit.status_item(kit.icon_bell, '2'),
	])
	return page('Status bar',
		'The bottom strip of a window: connection, context and quiet counters.', [
		demo('Window footer', framed(t, ui.div().flex_col().child(ui.div().h(60)).child(bar)),
			"kit.statusbar([kit.status_label(.running, 'Connected')], [kit.status_item('', 'UTF-8')])"),
	])
}

// commands is the demo command list for the palette (ctrl-k opens it anywhere).
pub fn commands() []input.Command {
	return [
		input.Command{'Mở tệp…', 'ctrl-o', fn (mut cx core.Cx) {
			kit.push_toast(mut cx, .info, 'Mở tệp')
		}},
		input.Command{'Lưu', 'ctrl-s', fn (mut cx core.Cx) {
			kit.push_toast(mut cx, .success, 'Đã lưu')
		}},
		input.Command{'Đổi giao diện sáng/tối', '', fn (mut cx core.Cx) {
			t := kit.theme_of(cx.app)
			kit.set_theme(mut cx.app, if t.mode == .dark { kit.light() } else { kit.dark() })
		}},
		input.Command{'Mở profiler', 'ctrl-shift-p', fn (mut cx core.Cx) {
			cx.win.debug.profiler = !cx.win.debug.profiler
			cx.win.invalidate_paint()
		}},
	]
}

// app_menu is the titlebar's hamburger menu: the palette, then the same commands.
fn app_menu() []kit.MenuItem {
	mut items := [
		kit.MenuItem{
			label:     'Command Palette'
			shortcut:  'ctrl-k'
			on_select: fn (mut cx core.Cx) {
				input.open_command_palette(mut cx, commands())
			}
		},
		kit.separator(),
	]
	for c in commands() {
		items << kit.MenuItem{
			label:     c.title
			shortcut:  c.keys
			on_select: c.run
		}
	}
	return items
}

fn palette_page(_ &kit.Theme) core.Element {
	return page('Command palette',
		'Fuzzy search over every command, with or without diacritics. Ctrl-K opens it anywhere.', [
		demo('Open', row([
			kit.button('pal-open').label('Mở bảng lệnh').icon(kit.icon_command).on_click(core.handler(fn (_ &core.ClickEvent, mut cx core.Cx) {
				input.open_command_palette(mut cx, commands())
			})),
			kit.kbd('ctrl-k'),
		]), 'input.open_command_palette(mut cx, commands)'),
	])
}
