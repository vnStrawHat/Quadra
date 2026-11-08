// Choice pages: checkbox, switch, radio group, segmented control, slider.
module pages

import quadra.core
import quadra.kit
import quadra.ui

pub struct CheckboxPage {
mut:
	terms  bool
	checks []bool = [true, true, false]
}

@[markused]
pub fn (mut p CheckboxPage) render(mut cx core.ViewCx[CheckboxPage]) core.Element {
	names := ['nginx', 'redis', 'postgres']
	mut list := ui.div().flex_col().gap(8)
	for i, n in names {
		list.child(kit.checkbox('cb-${n}', p.checks[i]).label(n).on_change(core.listener_with[CheckboxPage, int, bool](cx.entity,
			i, fn (mut p CheckboxPage, i int, v &bool, mut _ core.ViewCx[CheckboxPage]) {
			p.checks[i] = *v
		})))
	}
	all_on := p.checks.all(it)
	some := p.checks.any(it)
	return page('Checkbox', 'Turns one option on or off, or picks several from a list.', [
		demo('Basic', kit.checkbox('cb-terms', p.terms).label('Tôi đồng ý với điều khoản').on_change(cx.listener(fn (mut p CheckboxPage, v &bool, mut _ core.ViewCx[CheckboxPage]) {
			p.terms = *v
		})), "kit.checkbox('agree', v.agree).label('...').on_change(cx.listener(...))"),
		demo('Select several', col([
			kit.checkbox('cb-all', all_on).label('All services').indeterminate(some && !all_on).on_change(cx.listener(fn (mut p CheckboxPage, v &bool, mut _ core.ViewCx[CheckboxPage]) {
				for i in 0 .. p.checks.len {
					p.checks[i] = *v
				}
			})),
			ui.div().pl(24).child(list),
		]), '.indeterminate(some && !all)'),
		demo('States', row([
			kit.checkbox('cb-off', false).label('Off'),
			kit.checkbox('cb-on', true).label('On'),
			kit.checkbox('cb-mixed', false).label('Mixed').indeterminate(true),
			kit.checkbox('cb-dis', true).label('Disabled').disabled(true),
		]), ''),
	])
}

pub struct SwitchPage {
mut:
	wifi   bool = true
	follow bool
	dark   bool
}

@[markused]
pub fn (mut p SwitchPage) render(mut cx core.ViewCx[SwitchPage]) core.Element {
	t := kit.theme(cx)
	return page('Switch', 'Applies a setting immediately, without a Save button.', [
		demo('Basic', kit.switch('sw-wifi', p.wifi).label('Wi-Fi').on_change(cx.listener(fn (mut p SwitchPage, v &bool, mut _ core.ViewCx[SwitchPage]) {
			p.wifi = *v
		})), "kit.switch('wifi', v.wifi).label('Wi-Fi').on_change(...)"),
		demo('Settings', ui.div().flex_col().max_w(520).child(setting(t, 'Follow logs',
			'Scroll to new lines as they arrive', kit.switch('sw-follow', p.follow).on_change(cx.listener(fn (mut p SwitchPage, v &bool, mut _ core.ViewCx[SwitchPage]) {
			p.follow = *v
		})))).child(setting(t, 'Dark theme', 'Applies to every window', kit.switch('sw-dark',
			t.mode == .dark).on_change(cx.listener(fn (mut p SwitchPage, v &bool, mut cx core.ViewCx[SwitchPage]) {
			kit.set_theme(mut cx.app, if *v { kit.dark() } else { kit.light() })
		})))).child(setting(t, 'Telemetry', 'Managed by your organization', kit.switch('sw-dis',
			false).disabled(true))), ''),
	])
}

pub struct RadioPage {
mut:
	policy int = 1
	size   int
}

@[markused]
pub fn (mut p RadioPage) render(mut cx core.ViewCx[RadioPage]) core.Element {
	return page('Radio group',
		'Exactly one choice among a few visible options. Arrow keys move the selection.', [
		demo('Vertical', kit.radio_group('rg-policy', ['Always', 'IfNotPresent', 'Never'], p.policy).on_change(cx.listener(fn (mut p RadioPage, v &int, mut _ core.ViewCx[RadioPage]) {
			p.policy = *v
		})), "kit.radio_group('pull', ['Always', 'IfNotPresent', 'Never'], v.pull).on_change(...)"),
		demo('Horizontal', kit.radio_group('rg-size', ['Nhỏ', 'Vừa', 'Lớn'], p.size).horizontal().on_change(cx.listener(fn (mut p RadioPage, v &int, mut _ core.ViewCx[RadioPage]) {
			p.size = *v
		})), '.horizontal()'),
		demo('Disabled',
			kit.radio_group('rg-dis', ['Read', 'Write'], 0).horizontal().disabled(true), ''),
	])
}

pub struct SegmentedPage {
mut:
	view   int
	range  int = 1
	filter int
}

@[markused]
pub fn (mut p SegmentedPage) render(mut cx core.ViewCx[SegmentedPage]) core.Element {
	return page('Segmented control', 'Switches between views of the same content.', [
		demo('Views', kit.segmented('sg-view', ['List', 'Board', 'Calendar'], p.view).on_change(cx.listener(fn (mut p SegmentedPage, v &int, mut _ core.ViewCx[SegmentedPage]) {
			p.view = *v
		})), "kit.segmented('view', ['List', 'Board', 'Calendar'], v.view).on_change(...)"),
		demo('Time range', kit.segmented('sg-range', ['5m', '1h', '24h', '7d'], p.range).on_change(cx.listener(fn (mut p SegmentedPage, v &int, mut _ core.ViewCx[SegmentedPage]) {
			p.range = *v
		})), ''),
		demo('Medium', kit.segmented('sg-filter', ['All', 'Installed', 'Not Installed'], p.filter).size(.md).on_change(cx.listener(fn (mut p SegmentedPage, v &int, mut _ core.ViewCx[SegmentedPage]) {
			p.filter = *v
		})), '.size(.md)'),
	])
}

pub struct SliderPage {
mut:
	volume f32       = 40
	temp   f32       = 21
	price  kit.Range = kit.Range{200, 650}
}

@[markused]
pub fn (mut p SliderPage) render(mut cx core.ViewCx[SliderPage]) core.Element {
	t := kit.theme(cx)
	return page('Slider',
		'Drag, click the track, or use the arrow keys, Page Up/Down, Home and End.', [
		demo('Continuous', row([kit.icon(kit.icon_minus).size(14).color(t.p.text_3),
			kit.slider('sl-vol', p.volume).width(260).on_change(cx.listener(fn (mut p SliderPage, v &f32, mut _ core.ViewCx[SliderPage]) {
				p.volume = *v
			})),
			kit.icon(kit.icon_plus).size(14).color(t.p.text_3),
			mono(t, '${int(p.volume)}%')]), "kit.slider('volume', v.volume).on_change(...)"),
		demo('Stepped', row([
			kit.slider('sl-temp', p.temp).range(16, 30, 0.5).width(260).on_change(cx.listener(fn (mut p SliderPage, v &f32, mut _ core.ViewCx[SliderPage]) {
				p.temp = *v
			})),
			mono(t, '${p.temp:.1f} °C'),
		]), '.range(16, 30, 0.5)'),
		demo('Range', row([
			kit.range_slider('sl-price', p.price.lo, p.price.hi).range(0, 1000, 10).width(260).on_change(cx.listener(fn (mut p SliderPage, r &kit.Range, mut _ core.ViewCx[SliderPage]) {
				p.price = *r
			})),
			mono(t, '${int(p.price.lo)} – ${int(p.price.hi)} k₫'),
		]), "kit.range_slider('price', v.lo, v.hi).on_change(...)"),
		demo('Disabled', kit.slider('sl-dis', 60).width(260).disabled(true), ''),
	])
}
