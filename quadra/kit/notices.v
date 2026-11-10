// Inline notices and placeholders: alert (a tinted message in the page flow), empty_state
// (what an empty list or search shows), description_list (term and value pairs).
module kit

import quadra.core
import quadra.layout
import quadra.ui

fn tone_icon(tone Tone) string {
	return match tone {
		.success { icon_circle_check }
		.warning { icon_triangle_alert }
		.danger { icon_circle_alert }
		else { icon_info }
	}
}

// alert is an inline message (specs/15): a 14 px icon in the tone's color, the title and a
// muted 12 px message, on a faint tint of the tone with a 1 px border of it. Errors and
// warnings are announced (role alert).
pub fn alert(tone Tone, title string, message string) &Styled {
	return themed_with(AlertArgs{tone, title, message}, fn (a AlertArgs, t &Theme) core.Element {
		tone, title, message := a.tone, a.title, a.message
		c := t.tone(tone)
		mut text := ui.div().flex_col().gap(2).grow()
		if title != '' {
			text.child(ui.text(title))
		}
		if message != '' {
			text.child(ui.text(message).size(t.ui_text(.sm)).color(t.p.text_2))
		}
		return ui.div().flex_row().items_start().gap(8).p(8).rounded(t.radius_md).bg(c.alpha(0.1)).border(1).border_color(c.alpha(0.2)).text_size(t.ui_text(.md)).role(if tone in [
			.danger,
			.warning,
		] {
			core.Role.alert
		} else {
			core.Role.group
		}).label(if title != '' { title } else { message }).child(ui.div().h(23).center().child(icon(tone_icon(tone)).size(14).color(c))).child(text)
	})
}

// empty_state fills an empty area quietly (specs/15): a muted icon, the title, a muted
// hint and an optional action (pass ui.div() for none).
pub fn empty_state(glyph string, title string, description string, action core.Element) &Styled {
	return themed_with(EmptyArgs{glyph, title, description, action}, fn (a EmptyArgs, t &Theme) core.Element {
		glyph, title, description, action := a.glyph, a.title, a.description, a.action
		mut col := ui.div().flex_col().items_center().gap(4).p(32).w_full()
		col.child(icon(glyph).size(16).color(t.p.text_3))
		col.child(ui.text(title).color(t.p.text))
		if description != '' {
			col.child(ui.text(description).size(t.ui_text(.sm)).color(t.p.text_2))
		}
		return col.child(ui.div().pt(8).child(action))
	})
}

pub struct Description {
pub:
	term   string
	detail core.Element
}

// description_list shows term and detail pairs in two aligned columns.
pub fn description_list(items []Description) &Styled {
	return themed_with(items, fn (items []Description, t &Theme) core.Element {
		mut grid :=
			ui.div().cols([layout.max_content(), layout.fr(1)]).gap_x(24).gap_y(8).text_size(t.ui_text(.md))
		for it in items {
			grid.child(ui.text(it.term).color(t.p.text_2))
			grid.child(it.detail)
		}
		return grid
	})
}

struct AlertArgs {
	tone    Tone
	title   string
	message string
}

struct EmptyArgs {
	glyph       string
	title       string
	description string
	action      core.Element
}
