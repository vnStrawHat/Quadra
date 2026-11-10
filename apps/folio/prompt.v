// The save prompt: closing a tab, or the window, with unsaved changes asks Save, Don't
// save or Cancel (specs/21, Files).
module main

import quadra.core
import quadra.kit
import quadra.ui

@[heap]
struct Prompt {
mut:
	modal  int
	ids    []int // the tabs with unsaved changes
	window bool  // the window is closing, not one tab
}

// CloseAnswer is a prompt button: 0 cancel, 1 don't save, 2 save.
struct CloseAnswer {
	prompt &Prompt = unsafe { nil }
	pick   int
}

fn (mut f Folio) ask_save(ids []int, window bool, mut cx core.ViewCx[Folio]) {
	mut names := []string{}
	for id in ids {
		if t := f.find(id) {
			names << t.name()
		}
	}
	msg := if names.len == 1 {
		'${names[0]} has unsaved changes. Save them first?'
	} else {
		'${names.len} documents have unsaved changes (${names.join(', ')}). Save them first?'
	}
	ent := cx.entity
	mut pr := &Prompt{
		ids:    ids
		window: window
	}
	pr.modal = kit.open_modal(mut cx.Cx, kit.ModalOptions{
		title:       'Save changes?'
		width:       440
		body_with:   kit.build_with(msg, fn (msg string, _ &kit.Theme) core.Element {
			return ui.text(msg)
		})
		footer_with: kit.build_with(PromptRef{ent, pr}, fn (r PromptRef, _ &kit.Theme) core.Element {
			ent, pr := r.ent, r.pr
			return ui.div().flex_row().gap(8).child(answer_button(ent, pr, 0, 'close-cancel',
				'Cancel')).child(answer_button(ent, pr, 1, 'close-discard', "Don't save")).child(answer_button(ent,
				pr, 2, 'close-save', 'Save'))
		})
	})
}

struct PromptRef {
	ent u32
	pr  &Prompt = unsafe { nil }
}

fn answer_button(ent u32, pr &Prompt, pick int, id string, label string) core.Element {
	mut b := kit.button(id).label(label).on_click(core.listener_with[Folio, CloseAnswer, core.ClickEvent](ent, CloseAnswer{pr, pick},
		answer_close))
	if pick == 0 {
		b = b.ghost()
	} else if pick == 2 {
		b = b.primary()
	}
	return b
}

fn answer_close(mut f Folio, a CloseAnswer, _ &core.ClickEvent, mut cx core.ViewCx[Folio]) {
	kit.close_modal(mut cx.Cx, a.prompt.modal)
	if a.pick == 0 {
		return
	}
	for id in a.prompt.ids {
		mut t := f.find(id) or { continue }
		if a.pick == 2 {
			if !f.save_tab(mut t, mut cx) {
				return
			}
		} else {
			t.saved = t.text // let it go unsaved
		}
		if !a.prompt.window {
			f.drop_tab(id, mut cx)
		}
	}
	if a.prompt.window {
		cx.close_window()
	}
}
