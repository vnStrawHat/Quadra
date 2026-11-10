// Editor: the code editor view (specs/18). The Surface element paints it (surface.v),
// actions.v maps keys to Doc commands, mouse.v and find_bar.v handle the rest.
module code

import quadra.core
import quadra.prim
import quadra.kit
import quadra.kit.input
import quadra.text
import quadra.ui

pub enum Mark {
	none
	add
	change
	remove
}

pub enum DragMode {
	none
	char
	word
	line
	column
}

@[heap]
pub struct Editor {
pub mut:
	id           string
	doc          &Doc         = unsafe { nil }
	hl           &Highlighter = unsafe { nil }
	find         Find
	find_open    bool
	readonly     bool
	line_numbers bool = true
	wrap         bool         // soft wrap at the view width (wrap.v)
	marks        map[int]Mark // diff marks by line
	focus        core.FocusHandle
	change       ?core.AnyListener
	// view state, written by prepaint and the handlers
	scroll         prim.Point
	bounds         prim.Rect // the text area (right of the gutter)
	gutter         f32
	cell           prim.Size // the mono advance and the line height
	base           f32       // baseline offset inside a line
	style          text.TextStyle
	show_replace   bool
	reveal         bool // keep the primary cursor visible at the next prepaint
	widest         f32  // widest line painted so far (horizontal scroll extent)
	preedit        string
	preedit_cursor int
	last_edit      f64
	drag           DragMode
	drag_a         int // the word or line range the drag started on
	drag_e         int
	query          ?core.View[input.TextInput]
	replace_with   ?core.View[input.TextInput]
mut:
	rows     Rows              // the visual rows while wrapped
	link     ?core.AnyListener // on_link (links.v)
	hover    LinkHover         // the URL under the pointer
	link_hbs []int             // its hitboxes at the last prepaint
}

@[params]
pub struct EditorOptions {
pub:
	id       string = 'editor'
	text     string
	path     string // picks the tokenizer and comment style
	readonly bool
	tab_size int = 4
	use_tabs bool
	wrap     bool
}

pub fn new_editor(mut cx core.Cx, opts EditorOptions) core.View[Editor] {
	install(mut cx.app) or {}
	mut d := new_doc(opts.text)
	d.tab_size = opts.tab_size
	d.use_tabs = opts.use_tabs || opts.text.contains('\n\t')
	if opts.path.ends_with('.yaml') || opts.path.ends_with('.yml') {
		d.comment = '#'
	}
	return cx.new_view(Editor{
		id:       opts.id
		doc:      d
		hl:       new_highlighter(tokenizer_for(opts.path))
		readonly: opts.readonly
		wrap:     opts.wrap
		focus:    cx.focus_handle()
	})
}

pub fn (e &Editor) value() string {
	return e.doc.buf.file_text()
}

// set_text replaces the document (a new file: undo starts over).
pub fn set_text(mut cx core.Cx, ed core.View[Editor], s string) {
	mut w := cx.peek(ed.model())
	mut d := new_doc(s)
	d.tab_size, d.use_tabs, d.comment = w.doc.tab_size, w.doc.use_tabs, w.doc.comment
	w.doc = d
	w.hl = new_highlighter(w.hl.tok)
	w.scroll = prim.Point{}
	w.widest = 0
	w.find.version = -1 // a new document starts at version 0 too: drop the old matches
	cx.touch(ed.model())
}

// on_change calls l with the document after every edit (read it, do not keep it).
pub fn on_change(mut cx core.Cx, ed core.View[Editor], l core.Listener[Doc]) {
	mut w := cx.peek(ed.model())
	w.change = l.any
}

// edited runs after a command: reveal the cursor, restart the blink, report changes.
fn (mut e Editor) edited(version int, mut cx core.ViewCx[Editor]) {
	e.reveal = true
	e.last_edit = cx.now_ms()
	if e.doc.version != version {
		if l := e.change {
			cx.app.invoke(l, e.doc, cx.win, true)
		}
	}
	cx.notify()
}

@[markused]
pub fn (mut e Editor) render(mut cx core.ViewCx[Editor]) core.Element {
	mut d :=
		ui.div().id(e.id).test_id(e.id).size_full().flex_col().cursor(.ibeam).role(.textbox).label(e.id).key_context('Editor').track_focus(e.focus)
	d.on_action(cx.listener(fn (mut e Editor, a &EditorAction, mut cx core.ViewCx[Editor]) {
		e.run(a, mut cx)
	}))
	d.on_text_input(cx.listener(fn (mut e Editor, ev &core.TextInputEvent, mut cx core.ViewCx[Editor]) {
		e.type_text(ev.text, mut cx)
	}))
	d.on_ime(cx.listener(fn (mut e Editor, ev &core.ImeInputEvent, mut cx core.ViewCx[Editor]) {
		e.on_ime(ev, mut cx)
	}))
	d.on_mouse_down(cx.listener(fn (mut e Editor, ev &core.MouseDownEvent, mut cx core.ViewCx[Editor]) {
		e.mouse_down(ev, mut cx)
	}))
	d.on_mouse_move(cx.listener(fn (mut e Editor, ev &core.MouseMoveEvent, mut cx core.ViewCx[Editor]) {
		e.mouse_drag(ev, mut cx)
	}))
	d.on_scroll(cx.listener(fn (mut e Editor, ev &core.ScrollEvent, mut cx core.ViewCx[Editor]) {
		e.wheel(ev, mut cx)
	}))
	d.on_focus_change(cx.listener(fn (mut e Editor, ev &core.FocusChangeEvent, mut cx core.ViewCx[Editor]) {
		e.doc.break_group()
		e.preedit = ''
		cx.win.pw.enable_ime(ev.focused)
	}))
	d.child(&Surface{
		e: &e
	})
	if e.find_open {
		d.child(e.find_bar(mut cx))
	}
	return d
}

fn (mut e Editor) type_text(s string, mut cx core.ViewCx[Editor]) {
	// Control characters (tab, enter) arrive as keys and run as actions.
	if e.readonly || s == '' || (s.len == 1 && s[0] < 0x20) {
		return
	}
	v := e.doc.version
	e.doc.insert_text(s, if s.len <= 4 { 'type' } else { 'paste' })
	cx.stop_propagation()
	e.edited(v, mut cx)
}

fn (mut e Editor) on_ime(ev &core.ImeInputEvent, mut cx core.ViewCx[Editor]) {
	match ev.kind {
		.start, .end {
			e.preedit = ''
		}
		.preedit {
			e.preedit = ev.text
			e.preedit_cursor = ev.cursor
		}
		.commit {
			e.preedit = ''
			e.type_text(ev.text, mut cx)
		}
	}

	cx.notify()
}

fn (mut e Editor) wheel(ev &core.ScrollEvent, mut cx core.ViewCx[Editor]) {
	sideways := ev.mods.has(.shift)
	dx := if ev.dx == 0 && sideways { ev.dy } else { ev.dx }
	dy := if sideways { f32(0) } else { ev.dy }
	before := e.scroll
	e.scroll = prim.Point{e.scroll.x - dx, e.scroll.y - dy}
	e.clamp_scroll()
	if e.scroll != before {
		cx.stop_propagation()
		cx.notify()
	}
}

fn (mut e Editor) clamp_scroll() {
	max_y := prim.max(0, f32(e.row_count()) * e.cell.height - e.bounds.height + e.cell.height * 2)
	max_x := if e.wrap { f32(0) } else { prim.max(0, e.widest - e.bounds.width + e.cell.width * 4) }
	e.scroll = prim.Point{prim.clamp(e.scroll.x, 0, max_x), prim.clamp(e.scroll.y, 0, max_y)}
}

// page is the number of whole rows the view shows.
fn (e &Editor) page() int {
	return if e.cell.height > 0 { int(e.bounds.height / e.cell.height) - 1 } else { 20 }
}

pub fn syntax_color(s &kit.Syntax, k TokenKind) prim.Color {
	return match k {
		.keyword { s.keyword }
		.control { s.control }
		.str { s.str }
		.number { s.number }
		.constant { s.constant }
		.typ { s.typ }
		.function { s.function }
		.variable { s.variable }
		.property { s.property }
		.operator { s.operator }
		.punctuation { s.punctuation }
		.comment { s.comment }
		.tag { s.tag }
		.attribute { s.attribute }
		.regexp { s.regexp }
		.escape { s.escape }
		.invalid { s.invalid }
	}
}
