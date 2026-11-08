// TextInput: the single- and multi-line text field view (input, textarea, password).
module input

import quadra.core
import quadra.prim
import quadra.kit
import quadra.text
import quadra.ui

@[heap]
pub struct TextInput {
pub mut:
	id             string
	buf            Buffer
	hist           History
	placeholder    string
	password       bool
	multiline      bool
	rows           int = 1
	max_rows       int
	fill           bool // grow to fill the parent (editors)
	bare           bool // no box (InputOptions.bare)
	disabled       bool
	readonly       bool
	invalid        bool   // the border shows the error color (set_invalid)
	prefix         string // icon body (kit.icon_*)
	focus          core.FocusHandle
	preedit        string
	preedit_cursor int
	change         ?core.AnyListener
	submit         ?core.AnyListener
	filter         ?fn (s string) string // e.g. digits only for number inputs
	// layout state, written during prepaint
	style             text.TextStyle
	scroll_x          f32
	field_bounds      prim.Rect
	last_edit         f64
	placeholder_color prim.Color
	selection_color   prim.Color
	caret_color       prim.Color
	dragging          bool
}

@[params]
pub struct InputOptions {
pub:
	id          string
	value       string
	placeholder string
	password    bool
	multiline   bool
	rows        int = 1
	max_rows    int
	fill        bool // grow to fill the parent along its main axis (editors)
	bare        bool // no box: a field inside another surface (palette, panel filter)
	prefix      string
	disabled    bool
	readonly    bool
}

// new_input creates an input view owned by the current view; place it with cx.child_view.
pub fn new_input(mut cx core.Cx, opts InputOptions) core.View[TextInput] {
	install(mut cx.app) or {}
	mut b := Buffer{}
	b.set_text(opts.value)
	return cx.new_view(TextInput{
		id:          opts.id
		buf:         b
		placeholder: opts.placeholder
		password:    opts.password
		multiline:   opts.multiline
		rows:        opts.rows
		max_rows:    opts.max_rows
		fill:        opts.fill
		bare:        opts.bare
		prefix:      opts.prefix
		disabled:    opts.disabled
		readonly:    opts.readonly
		focus:       cx.focus_handle()
	})
}

pub fn textarea(mut cx core.Cx, opts InputOptions) core.View[TextInput] {
	return new_input(mut cx, InputOptions{
		...opts
		multiline: true
		rows:      if opts.rows > 1 { opts.rows } else { 3 }
	})
}

pub fn (v &TextInput) value() string {
	return v.buf.text
}

// set_value replaces the text (caret at the end) from outside the input. It starts a new
// document: undo cannot go back to the previous text.
pub fn set_value(mut cx core.Cx, input core.View[TextInput], s string) {
	mut w := cx.peek(input.model())
	w.buf.set_text(s)
	w.hist = History{}
	cx.touch(input.model())
}

// set_placeholder changes the hint shown while the input is empty (language switches).
pub fn set_placeholder(mut cx core.Cx, input core.View[TextInput], s string) {
	mut w := cx.peek(input.model())
	w.placeholder = s
	cx.touch(input.model())
}

// set_invalid marks the input invalid (specs/15): its border turns the error color, also
// while focused. Call it where validation runs, not while rendering.
pub fn set_invalid(mut cx core.Cx, input core.View[TextInput], on bool) {
	mut w := cx.peek(input.model())
	if w.invalid != on {
		w.invalid = on
		cx.touch(input.model())
	}
}

// on_change / on_submit wire the input to its owner's listeners.
pub fn on_change(mut cx core.Cx, input core.View[TextInput], l core.Listener[string]) {
	mut w := cx.peek(input.model())
	w.change = l.any
}

pub fn on_submit(mut cx core.Cx, input core.View[TextInput], l core.Listener[string]) {
	mut w := cx.peek(input.model())
	w.submit = l.any
}

// shown is the text as displayed (bullets for passwords).
pub fn (v &TextInput) shown() string {
	if !v.password {
		return v.buf.text
	}
	return '•'.repeat(text.boundaries(v.buf.text).len - 1)
}

pub fn (v &TextInput) display_index(i int) int {
	if !v.password {
		return i
	}
	return (text.boundaries(v.buf.text[..i]).len - 1) * '•'.len
}

pub fn (v &TextInput) buffer_index(d int) int {
	if !v.password {
		return d
	}
	bs := text.boundaries(v.buf.text)
	k := d / '•'.len
	return if k < bs.len { bs[k] } else { v.buf.text.len }
}

@[markused]
pub fn (mut v TextInput) render(mut cx core.ViewCx[TextInput]) core.Element {
	t := kit.theme(cx)
	c := t.colors
	v.style = text.TextStyle{
		family:      t.font_family
		size:        t.font_size(.md)
		color:       if v.disabled { t.p.text_disabled } else { t.code.editor.foreground }
		line_height: t.line_height
	}
	v.placeholder_color = t.p.text_3
	v.selection_color = c.selection
	v.caret_color = t.code.editor.cursor
	focused := v.focus.is_focused(cx.Cx)
	// The border shows focus (specs/15), so the focus mark is off.
	mut d := ui.div().id(v.id).test_id(v.id).flex_row().items(if v.multiline {
		.start
	} else {
		.center
	}).gap(4).px(8).py(if v.multiline { 8 } else { 6 }).min_h(32).rounded(t.radius_lg).border(1).border_color(if v.invalid {
		t.p.error
	} else if focused {
		t.p.border_focused
	} else {
		t.p.line
	}).bg(if v.disabled { t.p.panel } else { t.p.canvas }).focus_ring(ui.Patch{}).cursor(.ibeam).role(.textbox).label(if v.placeholder != '' {
		v.placeholder
	} else {
		v.id
	}).sem_value(if v.password { '' } else { v.buf.text }).disabled(v.disabled)
	d.key_context(if v.multiline { 'TextInput Multiline' } else { 'TextInput' })
	if v.fill {
		// A filling input is an editor: a plane, not a boxed control (specs/15).
		d.grow().border(0).rounded(0).px(24).py(16)
	} else if v.bare {
		d.border(0).rounded(0).px(0).py(0).min_h(0).bg(t.p.canvas.alpha(0)) // the caret shows focus
	}
	if !v.disabled {
		d.track_focus(v.focus)
		d.on_action(cx.listener(fn (mut v TextInput, e &Edit, mut cx core.ViewCx[TextInput]) {
			v.apply(e.op, mut cx)
		}))
		d.on_text_input(cx.listener(fn (mut v TextInput, e &core.TextInputEvent, mut cx core.ViewCx[TextInput]) {
			v.type_text(e.text, mut cx)
		}))
		d.on_ime(cx.listener(fn (mut v TextInput, e &core.ImeInputEvent, mut cx core.ViewCx[TextInput]) {
			v.on_ime(e, mut cx)
		}))
		d.on_mouse_down(cx.listener(fn (mut v TextInput, e &core.MouseDownEvent, mut cx core.ViewCx[TextInput]) {
			v.mouse_down(e, mut cx)
		}))
		d.on_mouse_move(cx.listener(fn (mut v TextInput, e &core.MouseMoveEvent, mut cx core.ViewCx[TextInput]) {
			v.mouse_drag(e, mut cx)
		}))
		d.on_focus_change(cx.listener(fn (mut v TextInput, e &core.FocusChangeEvent, mut cx core.ViewCx[TextInput]) {
			v.hist.break_run()
			v.preedit = ''
			cx.win.pw.enable_ime(e.focused)
		}))
	}
	if v.prefix != '' {
		d.child(kit.icon(v.prefix).color(c.muted_fg))
	}
	return d.child(&Field{
		v: &v
	})
}
