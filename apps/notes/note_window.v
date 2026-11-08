// A note in its own window. Both windows share the Notes model, so an edit here shows up
// in the main window's list (and editor, when it is not being typed in) at once.
module main

import quadra.app
import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.platform
import quadra.ui

pub struct NoteWindow {
pub mut:
	notes  core.Model[Notes]
	id     string
	editor core.View[input.TextInput]
	win    platform.WindowId // observers run without (or with another) window in cx
}

pub fn open_note_window(mut cx core.Cx, m core.Model[Notes], id string) ! {
	n := cx.peek(m)
	note := n.find(id) or { return error('no note ${id}') }
	cx.app.open_window(platform.WindowOptions{
		title:  title_of(note, app.t(cx, 'note.untitled'))
		width:  560
		height: 640
	}, NoteWindow{
		notes: m
		id:    id
	})!
}

@[markused]
pub fn (mut w NoteWindow) render(mut cx core.ViewCx[NoteWindow]) core.Element {
	if w.editor.id == 0 {
		w.win = cx.win.id
		body := if note := cx.peek(w.notes).find(w.id) { note.body } else { '' }
		w.editor = input.textarea(mut cx.Cx,
			id:    'window-editor'
			value: body
			rows:  20
			fill:  true
		)
		input.on_change(mut cx.Cx, w.editor, cx.listener(fn (mut w NoteWindow, body &string, mut cx core.ViewCx[NoteWindow]) {
			edit(mut cx.Cx, w.notes, w.id, *body)
		}))
		core.observe(mut cx, w.notes, fn (mut w NoteWindow, m core.Model[Notes], mut cx core.ViewCx[NoteWindow]) {
			note := cx.peek(m).find(w.id) or {
				if mut win := cx.app.window(w.win) { // deleted elsewhere: close this window
					win.pw.close()
				}
				return
			}
			ed := cx.peek(w.editor.model())
			if ed.value() != note.body && !ed.focus.is_focused_in(cx.app) {
				input.set_value(mut cx.Cx, w.editor, note.body)
			}
		})
	}
	t := kit.theme(cx.Cx)
	n := cx.read(w.notes)
	note := n.find(w.id) or { return ui.div() }
	meta := ui.div().flex_row().items_center().gap(8).px(24).h(t.panel_header_h + 4).shrink_0().bg(t.p.panel).border_b(1).border_color(t.p.line).text_size(t.font_size(.sm)).child(kit.icon(kit.icon_folder).size(14).color(t.p.text_3)).child(ui.text(note.folder).color(t.p.text_2)).child(ui.text('/').color(t.p.text_3)).child(ui.text(title_of(note, app.t(cx.Cx,
		'note.untitled'))).weight(600))
	return ui.div().flex_col().size_full().bg(t.p.canvas).child(meta).child(ui.div().flex_col().grow().child(cx.child_view(w.editor)))
}
