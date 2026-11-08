// Overview pages: getting started, typography, colors, icons.
module pages

import quadra.core
import quadra.prim
import quadra.kit
import quadra.ui

fn start_page(t &kit.Theme) core.Element {
	principles := [
		['Quiet chrome, loud content',
			'Panels and toolbars recede; code, logs and data get full contrast.'],
		['Planes, not boxes',
			'Regions differ by one step of tone and a hairline. Only overlays cast shadows.'],
		['Dense with rhythm',
			'Compact by default, on a 4 px grid and four control heights: 18, 22, 28, 32.'],
		['Mono is first-class', 'Identifiers, numbers, paths and durations use the mono face.'],
		['Status by color and shape', 'Every status has a shape too, so it reads in grayscale.'],
		['One accent',
			'The accent colors text and icons: toggled, matched, linked. Selection is a neutral step.'],
	]
	mut list := ui.div().flex_col()
	for i, p in principles {
		list.child(ui.div().flex_row().gap(16).py(10).border_b(1).border_color(t.p.line).child(ui.div().w(24).child(mono(t, '0${
			i + 1}'))).child(ui.div().flex_col().gap(2).child(ui.text(p[0]).weight(600)).child(hint(t,
			p[1]))))
	}
	return page('Quadra',
		'A native, GPU-rendered UI framework for V. Everything here is drawn by Quadra itself, in the One design language: built for terminals, editors and cluster tools that people keep open all day.', [
		demo('Hello, world', ui.text('Xin chào! Pick a component on the left.'),
			"ui.div().p(16).child(ui.text('Xin chào'))"),
		demo('Design principles', list, ''),
		demo('Developer shortcuts', row([kit.kbd('ctrl-k'), hint(t, 'commands'),
			kit.kbd('ctrl-shift-p'), hint(t, 'profiler'), kit.kbd('ctrl-shift-f'),
			hint(t, 'paint flashing'), kit.kbd('ctrl-shift-i'),
			hint(t, 'inspector')]), ''),
	])
}

fn typography_page(t &kit.Theme) core.Element {
	roles := [['Display', '23 / 600'], ['Headline', '18 / 400'],
		['Large', '16 / 400'], ['Body', '14 / 400'], ['Small', '12 / 400'],
		['XSmall', '10 / 400']]
	sizes := [t.text_2xl, t.text_xl, t.text_lg, t.text_md, t.text_sm, t.text_xs]
	mut scale := ui.div().flex_col()
	for i, r in roles {
		w := if i == 0 { 600 } else { 400 }
		scale.child(ui.div().flex_row().items_center().gap(16).py(8).border_b(1).border_color(t.p.line).child(ui.div().w(120).child(hint(t,
			'${r[0]} · ${r[1]}'))).child(ui.text('Tiếng Việt có dấu, rõ ràng').size(t.scaled(sizes[i])).weight(w)))
	}
	code := 'fn main() {\n    pods := cluster.list(ns: "default")!\n    println("\${pods.len} pods, 0O 1lI")\n}'
	para := 'Quadra shapes text with the platform engine and caches every word, so long paragraphs wrap in microseconds. Chữ tiếng Việt như “Đường Trường Sơn”, “nghiêng ngả”, “thuở ấy” hiển thị đúng dấu ở mọi cỡ chữ.'
	return page('Typography',
		'IBM Plex Sans for the interface, Lilex for code and values, both bundled. Weights are 400 and 600 only.', [
		demo('Scale', scale, "ui.text('...').size(t.scaled(t.text_lg))"),
		demo('Mono',
			ui.div().p(12).bg(t.code.terminal.background).child(ui.text(code).family(t.mono_family).color(t.code.syntax.variable).selectable()),
			'ui.text(src).family(t.mono_family)'),
		demo('Selectable paragraph',
			ui.div().max_w(560).child(ui.text(para).color(t.p.text_2).selectable()),
			'ui.text(s).selectable()  // drag, double-click a word, ctrl-a, ctrl-c'),
	])
}

fn swatches(t &kit.Theme, items map[string]prim.Color) core.Element {
	mut grid := ui.div().grid_fill(150).gap(12)
	for name, color in items {
		hex := if color.a >= 1 { color.hex()[..7] } else { color.hex() }
		grid.child(ui.div().flex_col().gap(4).child(ui.div().h(40).bg(color).border(1).border_color(t.p.line)).child(ui.text(name).size(t.scaled(t.text_sm))).child(ui.text(hex).family(t.mono_family).size(t.scaled(t.text_xs)).color(t.p.text_3)))
	}
	return grid
}

fn colors_page(t &kit.Theme) core.Element {
	p := t.p
	mut ansi := ui.div().flex_row()
	for c in t.code.terminal.ansi {
		ansi.child(ui.div().h(28).grow().bg(c))
	}
	return page('Colors',
		'Design roles of the current theme (${t.name}). Components read roles, never raw colors.', [
		demo('Planes', swatches(t, {
			'canvas':      p.canvas
			'panel':       p.panel
			'raised':      p.raised
			'overlay_bg':  p.overlay_bg
			'line':        p.line
			'line_strong': p.line_strong
		}), 't.p.panel'),
		demo('Text', swatches(t, {
			'text':          p.text
			'text_2':        p.text_2
			'text_3':        p.text_3
			'text_disabled': p.text_disabled
		}), ''),
		demo('Accent and feedback', swatches(t, {
			'accent':      p.accent
			'accent_tint': p.accent_tint
			'accent_text': p.accent_text
			'ok':          p.ok
			'warn':        p.warn
			'error':       p.error
			'info':        p.info
		}), ''),
		demo('Terminal (16 ANSI colors)', ansi, 't.code.terminal.ansi[1]  // red'),
		demo('Elevation: overlays only', row([
			ui.div().size(96).bg(p.overlay_bg).border(1).border_color(p.line_strong).rounded(t.radius_lg).shadow_level(1).center().child(hint(t,
				'menu')),
			ui.div().size(96).bg(p.overlay_bg).border(1).border_color(p.line_strong).rounded(t.radius_xl).shadow_level(2).center().child(hint(t,
				'dialog')),
		]), ''),
	])
}

fn icons_page(t &kit.Theme) core.Element {
	mut grid := ui.div().grid_fill(112).gap(4)
	for ic in icon_catalog {
		grid.child(ui.div().flex_col().items_center().gap(8).py(12).rounded(t.radius_md).hover(ui.Patch{
			background: t.p.panel_hover
		}).child(kit.icon(ic[1]).size(18).color(t.p.text)).child(ui.text(ic[0]).size(t.scaled(t.text_xs)).color(t.p.text_3)))
	}
	return page('Icons',
		'${icon_catalog.len} stroked 24 px icons, drawn as vectors at any size and tinted from the theme.', [
		demo('Catalog', grid, 'kit.icon(kit.icon_settings).size(16).color(t.p.text_2)'),
	])
}
