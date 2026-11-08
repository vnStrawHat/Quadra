// Display pages: status, badge, tag, kbd, avatar, progress, spinner, skeleton, card.
module pages

import quadra.core
import quadra.kit
import quadra.ui

fn status_page(t &kit.Theme) core.Element {
	pods := [
		['api-7d9f8c6b5-x2k4p', '0', '3d'],
		['worker-5c6d7f8-9qwer', '0', '14m'],
		['migrate-28417-hk2zd', '5', '2m'],
		['cron-backup-28417-1', '0', '1h'],
		['web-6b7c8d9e0-old01', '1', '6d'],
	]
	states := [kit.Status.running, .pending, .failed, .succeeded, .terminating]
	texts := ['Running', 'ContainerCreating', 'CrashLoopBackOff', 'Completed', 'Terminating']
	mut table := ui.div().flex_col().border(1).border_color(t.p.line)
	table.child(ui.div().flex_row().items_center().h(t.panel_header_h).px(12).bg(t.p.panel).border_b(1).border_color(t.p.line).text_size(t.scaled(t.text_sm)).text_color(t.p.text_2).child(ui.div().w(260).child(ui.text('Name'))).child(ui.div().w(200).child(ui.text('Status'))).child(ui.div().w(80).child(ui.text('Restarts'))).child(ui.text('Age')))
	for i, pod in pods {
		table.child(ui.div().flex_row().items_center().h(t.row_h + 2).px(12).border_b(1).border_color(t.p.line).hover(ui.Patch{
			background: t.p.panel_hover
		}).child(ui.div().w(260).child(ui.text(pod[0]).family(t.mono_family).selectable())).child(ui.div().w(200).child(kit.status_label(states[i],
			texts[i]))).child(ui.div().w(80).child(mono(t, pod[1]))).child(mono(t, pod[2])))
	}
	return page('Status',
		'Workload state as a color and a shape, so it reads in grayscale and for color-blind users.', [
		demo('States', row([kit.status_label(.running, ''), kit.status_label(.pending, ''),
			kit.status_label(.failed, ''), kit.status_label(.succeeded, ''),
			kit.status_label(.terminating, ''), kit.status_label(.unknown, '')]),
			"kit.status_label(.failed, 'CrashLoopBackOff')"),
		demo('In a table', table, ''),
		demo('Marks only', row([kit.status_mark(.running), kit.status_mark(.pending),
			kit.status_mark(.failed), kit.status_mark(.succeeded),
			kit.status_mark(.terminating), kit.status_mark(.unknown)]), 'kit.status_mark(.running)'),
	])
}

fn badge_page(_ &kit.Theme) core.Element {
	return page('Badge', 'A short label for a count or a state, tinted by tone.', [
		demo('Tones', row([kit.badge('Neutral', .neutral), kit.badge('New', .primary),
			kit.badge('Healthy', .success), kit.badge('Degraded', .warning),
			kit.badge('Down', .danger), kit.badge('Beta', .info)]),
			"kit.badge('Healthy', .success)"),
		demo('Counts', row([ui.div().flex_row().items_center().gap(6).child(ui.text('Problems')).child(kit.badge('12',
			.danger)),
			ui.div().flex_row().items_center().gap(6).child(ui.text('Warnings')).child(kit.badge('3',
				.warning)),
			ui.div().flex_row().items_center().gap(6).child(ui.text('Changes')).child(kit.badge('148',
				.neutral))]),
			''),
	])
}

fn tag_page(_ &kit.Theme) core.Element {
	return page('Tag', 'Metadata such as labels, versions and selectors.', [
		demo('Labels', row([kit.tag('app=api'), kit.tag('tier=backend'),
			kit.tag('env=prod'), kit.tag('v1.29.3')]), "kit.tag('app=api')"),
	])
}

fn kbd_page(t &kit.Theme) core.Element {
	keys := [['ctrl-k', 'Command palette'], ['ctrl-p', 'Go to file'],
		['ctrl-shift-p', 'Profiler'], ['ctrl-`', 'Toggle terminal'],
		['f2', 'Rename']]
	mut list := ui.div().flex_col().max_w(420)
	for k in keys {
		list.child(ui.div().flex_row().items_center().py(6).border_b(1).border_color(t.p.line).child(ui.div().grow().child(ui.text(k[1]))).child(kit.kbd(k[0])))
	}
	return page('Kbd', 'A keyboard shortcut, written once and shown with the platform key names.', [
		demo('Shortcuts', list, "kit.kbd('ctrl-shift-p')"),
	])
}

fn avatar_page(t &kit.Theme) core.Element {
	return page('Avatar', 'A person, as initials on a color derived from the name, or an image.', [
		demo('Sizes', row([kit.avatar('Nguyễn Văn An', 40, ''),
			kit.avatar('Trần Bình', 32, ''), kit.avatar('Lê Thị Cúc', 24, ''),
			kit.avatar('Ada Lovelace', 20, '')]), "kit.avatar('Nguyễn Văn An', 32, '')"),
		demo('Author line', ui.div().flex_row().items_center().gap(8).child(kit.avatar('Ada Lovelace',
			20, '')).child(ui.text('Ada Lovelace').weight(600)).child(hint(t,
			'committed 2 hours ago')).child(mono(t, 'a1b2c3d')), ''),
	])
}

fn progress_page(t &kit.Theme) core.Element {
	return page('Progress bar',
		'Known or unknown progress. Animations repaint only, they never re-render.', [
		demo('Determinate', col([ui.div().flex_row().items_center().gap(12).child(kit.progress(0.25).width(320)).child(mono(t,
			'25%')),
			ui.div().flex_row().items_center().gap(12).child(kit.progress(0.7).width(320)).child(mono(t,
				'70%'))]),
			'kit.progress(0.7).width(320)'),
		demo('Indeterminate', kit.progress_indeterminate().width(320),
			'kit.progress_indeterminate()'),
	])
}

fn spinner_page(t &kit.Theme) core.Element {
	return page('Spinner',
		'Work in progress of unknown length, inline with the text it belongs to.', [
		demo('Sizes', row([kit.spinner().size(12), kit.spinner().size(16),
			kit.spinner().size(24)]), 'kit.spinner().size(16)'),
		demo('Inline', ui.div().flex_row().items_center().gap(8).child(kit.spinner().size(12)).child(hint(t,
			'Pulling image nginx:1.27…')), ''),
	])
}

fn skeleton_page(_ &kit.Theme) core.Element {
	return page('Skeleton',
		'Placeholder shapes while content loads, in the layout of the real content.', [
		demo('Lines', col([kit.skeleton(240, 12), kit.skeleton(320, 12),
			kit.skeleton(180, 12)]), 'kit.skeleton(320, 12)'),
	])
}

fn card_page(t &kit.Theme) core.Element {
	return page('Card', 'Groups a few related values. Cards never nest; use a plane for regions.', [
		demo('Metrics', row([
			kit.card('CPU', 'cluster, 5 min', ui.text('42%').family(t.mono_family).size(22)),
			kit.card('Memory', 'cluster, 5 min', ui.text('11.6 GiB').family(t.mono_family).size(22)),
			kit.card('Pods', 'running / total', ui.text('118 / 124').family(t.mono_family).size(22)),
		]), "kit.card('CPU', 'cluster, 5 min', body)"),
	])
}

pub struct RichTextPage {
mut:
	clicked string
}

@[markused]
pub fn (mut p RichTextPage) render(mut cx core.ViewCx[RichTextPage]) core.Element {
	t := kit.theme(cx)
	para := ui.rich_text([ui.span('Pod '), ui.span('api-7d9f8c').family(t.mono_family).color(t.p.accent_text),
		ui.span(' was '), ui.span('restarted').bold(),
		ui.span(' 3 times in the last hour. The probe on '), ui.span('/healthz').family(t.mono_family),
		ui.span(' timed out; see the '), ui.span('troubleshooting guide').link('https://example.com/probes').color(t.p.accent_text),
		ui.span(' or the '), ui.span('events').link('events').color(t.p.accent_text),
		ui.span('.')]).on_link(cx.listener(fn (mut p RichTextPage, url &string, mut _ core.ViewCx[RichTextPage]) {
		p.clicked = *url
	}))
	return page('Rich text',
		'Mixed styles and links in one wrapped paragraph: words glued across spans never break apart.', [
		demo('Event detail', col([ui.div().w(420).child(para),
			hint(t, 'Clicked: ${p.clicked}')]),
			"ui.rich_text([ui.span('was '), ui.span('restarted').bold(), ui.span('guide').link(url)]).on_link(l)"),
	])
}
