// Command palette: fuzzy search over app commands, in a spotlight near the top of the
// window: the query on top, the matches below with their keys in muted text.
module input

import quadra.core
import quadra.prim
import quadra.kit
import quadra.ui

pub struct Command {
pub:
	title string
	keys  string // shortcut shown on the right, e.g. "ctrl-s"
	run   fn (mut cx core.Cx) = unsafe { nil }
	// on_run is run without a closure (`core.listener_with(view, data, f)`): a palette is
	// opened from a handler, where V frees no closure (specs/14). It gets `true`.
	on_run core.Listener[bool]
}

@[heap]
pub struct Palette {
pub mut:
	field     core.View[TextInput]
	commands  []Command
	matches   []int
	query     string
	highlight int
	modal     int
	focused   bool
}

// open_command_palette shows the palette in the current window.
pub fn open_command_palette(mut cx core.Cx, commands []Command) {
	mut root := core.Cx{
		app: cx.app
		win: cx.win
	}
	words := kit.strings_of(cx.app)
	field := new_input(mut root,
		id:          'palette-input'
		placeholder: words.command_placeholder
		bare:        true
	)
	pv := root.new_view(Palette{
		field:    field
		commands: commands
		matches:  []int{len: commands.len, init: index}
	})
	on_change(mut root, field, core.Listener[string]{core.AnyListener{
		view: pv.id
		call: palette_typed
	}})
	id := kit.open_modal(mut cx, kit.ModalOptions{
		side:      .top
		title:     words.commands
		width:     608
		body_with: kit.view_body(pv.any())
	})
	mut w := cx.peek(pv.model())
	w.modal = id
}

fn palette_typed(_ voidptr, view voidptr, event voidptr, mut _ core.Cx) {
	mut p := unsafe { &Palette(view) }
	q := unsafe { *(&string(event)) }
	p.query = q
	mut scored := []prim.Point{}
	for i, c in p.commands {
		if s := fuzzy(q, c.title) {
			scored << prim.pt(f32(s), f32(i))
		}
	}
	scored.sort_with_compare(fn (a &prim.Point, b &prim.Point) int {
		if a.x != b.x {
			return if a.x > b.x { -1 } else { 1 }
		}
		return int(a.y - b.y)
	})
	p.matches = scored.map(int(it.y))
	p.highlight = 0
}

fn (mut p Palette) run(mut cx core.ViewCx[Palette]) {
	if p.highlight < p.matches.len {
		cmd := p.commands[p.matches[p.highlight]]
		kit.close_modal(mut cx.Cx, p.modal)
		if cmd.run != unsafe { nil } {
			cmd.run(mut cx.Cx)
		} else {
			yes := true
			cx.app.invoke(cmd.on_run.any, &yes, cx.win, true)
		}
	}
}

@[markused]
pub fn (mut p Palette) render(mut cx core.ViewCx[Palette]) core.Element {
	t := kit.theme(cx)
	if !p.focused {
		p.focused = true
		field := cx.read_view(p.field)
		cx.win.set_focus(field.focus.id, true)
	}
	// The picker (specs/15): a 36 px query row over a hairline, rows inset 4 and 32 tall,
	// the highlight two steps up, matched characters in the accent, shortcuts muted on the
	// right.
	mut list := ui.div().flex_col().py(4).max_h(347).overflow_y_scroll().role(.listbox)
	for k, i in p.matches {
		c := p.commands[i]
		on := k == p.highlight
		mut row := ui.div().test_id('cmd-${i}').flex_row().items_center().gap(8).mx(4).px(6).h(32).rounded(t.radius_md).cursor(.pointer).role(.listitem).label(c.title).selected(on).on_click(core.listener_with[Palette, int, core.ClickEvent](cx.entity,
			k, fn (mut p Palette, k int, _ &core.ClickEvent, mut cx core.ViewCx[Palette]) {
			p.highlight = k
			p.run(mut cx)
		}))
		if on {
			row.bg(t.p.press)
		} else {
			row.hover(ui.Patch{
				background: t.p.panel_hover
			})
		}
		hits := fuzzy_hits(p.query, c.title) or { []int{} }
		row.child(ui.rich_text(marked(c.title, hits, t.p.accent_text))).child(ui.div().grow())
		if c.keys != '' {
			row.child(ui.text(kit.key_label(c.keys)).color(t.p.text_2))
		}
		list.child(row)
	}
	if p.matches.len == 0 {
		list.child(ui.div().px(10).py(4).child(ui.text(kit.strings_of(cx.app).no_commands).color(t.p.text_2)))
	}
	return ui.div().flex_col().w_full().on_key_down(cx.listener(fn (mut p Palette, e &core.KeyDownEvent, mut cx core.ViewCx[Palette]) {
		n := p.matches.len
		match e.key {
			.down {
				if n > 0 {
					p.highlight = (p.highlight + 1) % n
				}
			}
			.up {
				if n > 0 {
					p.highlight = (p.highlight - 1 + n) % n
				}
			}
			.enter {
				p.run(mut cx)
			}
			else {
				return
			}
		}

		cx.stop_propagation()
	})).child(ui.div().flex_row().items_center().h(37).px(10).border_b(1).border_color(t.p.line).child(ui.div().grow().child(cx.child_view(p.field)))).child(list)
}
