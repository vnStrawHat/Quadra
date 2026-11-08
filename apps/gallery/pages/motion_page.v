// Motion page: paint-only transforms and eased transitions on hover and press.
module pages

import quadra.core
import quadra.kit
import quadra.prim
import quadra.ui

fn motion_card(t &kit.Theme, id string, title string, hover ui.Patch, active ui.Patch) core.Element {
	return ui.div().test_id(id).w(150).h(90).p(14).rounded(t.radius_lg).bg(t.p.raised).border(1).border_color(t.p.line).cursor(.pointer).transition(160).hover(hover).active(active).child(ui.text(title).weight(600))
}

fn motion_dot(t &kit.Theme, m ui.Motion) core.Element {
	return ui.div().w(150).h(90).flex_row().items_center().justify_center().child(ui.div().size(22).rounded(5).bg(t.colors.primary).animate(m))
}

fn motion_page(t &kit.Theme) core.Element {
	return page('Motion',
		'Transforms move, scale and turn what a div paints without changing layout; transition eases a change of style, and animate plays a timeline of tweens in sequence or together.', [
		demo('Hover and press', row([
			motion_card(t, 'mo-lift', 'Lift', ui.Patch{
				translate:    prim.Point{0, -3}
				border_color: t.colors.primary
			}, ui.Patch{
				scale: 0.97
			}),
			motion_card(t, 'mo-grow', 'Grow', ui.Patch{
				scale: 1.06
			}, ui.Patch{
				scale: 1
			}),
			motion_card(t, 'mo-fade', 'Fade', ui.Patch{
				opacity:    0.55
				background: t.colors.primary.alpha(0.12)
			}, ui.Patch{}),
			motion_card(t, 'mo-turn', 'Turn', ui.Patch{
				rotate: -6
			}, ui.Patch{
				rotate: 6
			}),
		]), 'ui.div().transition(160).hover(ui.Patch{ translate: prim.Point{0, -3} })'),
		demo('Timelines', row([
			motion_dot(t, (ui.tween(380, ui.Patch{ translate: prim.Point{0, -18} }) + ui.tween_with(380, .ease_in, ui.Patch{
				translate: prim.Point{0, 0}
			})).forever()),
			motion_dot(t, ui.tween_with(1200, .linear, ui.Patch{ rotate: 360 }).forever()),
			motion_dot(t, ui.parallel(ui.tween(600, ui.Patch{ scale: 1.6 }), ui.tween(600, ui.Patch{
				opacity: 0.3
			})).repeat(2).alternate().forever()),
		]), '(ui.tween(380, up) + ui.tween(380, down)).forever(), ui.parallel(a, b).alternate()'),
	])
}
