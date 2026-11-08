// The formatting toolbar above the editor (Raw, Split): a heading menu, inline marks, link,
// lists and quote, and an insert menu for blocks. Every button is a Cmd, so the same
// actions have keys and palette entries (specs/21, Toolbar).
module main

import quadra.core
import quadra.kit
import quadra.ui

const format_commands = [
	CmdInfo{.bold, 'Format: Bold', 'ctrl-b'},
	CmdInfo{.italic, 'Format: Italic', 'ctrl-i'},
	CmdInfo{.strike, 'Format: Strikethrough', 'ctrl-shift-x'},
	CmdInfo{.code, 'Format: Inline code', 'ctrl-e'},
	CmdInfo{.math, 'Format: Inline math', 'ctrl-shift-m'},
	CmdInfo{.link, 'Insert: Link', 'ctrl-k'},
	CmdInfo{.para, 'Format: Normal text', 'ctrl-alt-0'},
	CmdInfo{.h1, 'Format: Heading 1', 'ctrl-alt-1'},
	CmdInfo{.h2, 'Format: Heading 2', 'ctrl-alt-2'},
	CmdInfo{.h3, 'Format: Heading 3', 'ctrl-alt-3'},
	CmdInfo{.bullet, 'Format: Bulleted list', 'ctrl-shift-8'},
	CmdInfo{.numbered, 'Format: Numbered list', 'ctrl-shift-7'},
	CmdInfo{.task, 'Format: Task list', 'ctrl-shift-9'},
	CmdInfo{.quote, 'Format: Quote', 'ctrl-shift-q'},
	CmdInfo{.image, 'Insert: Image', ''},
	CmdInfo{.code_block, 'Insert: Code block', ''},
	CmdInfo{.math_block, 'Insert: Math block', ''},
	CmdInfo{.diagram, 'Insert: Mermaid diagram', ''},
	CmdInfo{.table, 'Insert: Table', ''},
	CmdInfo{.rule, 'Insert: Horizontal rule', ''},
]

struct Tool {
	cmd   Cmd
	glyph string
	tip   string
}

// The buttons, in groups with a divider between them.
const tool_groups = [
	[Tool{.bold, kit.icon_bold, 'Bold'}, Tool{.italic, kit.icon_italic, 'Italic'},
		Tool{.strike, kit.icon_strikethrough, 'Strikethrough'},
		Tool{.code, kit.icon_code, 'Inline code'}, Tool{.math, kit.icon_sigma, 'Inline math'}],
	[Tool{.link, kit.icon_link, 'Link'}],
	[Tool{.bullet, kit.icon_list, 'Bulleted list'},
		Tool{.numbered, kit.icon_list_ordered, 'Numbered list'},
		Tool{.task, kit.icon_list_checks, 'Task list'}, Tool{.quote, kit.icon_text_quote, 'Quote'}],
]

const insert_tools = [Tool{.image, kit.icon_image, 'Image'},
	Tool{.code_block, kit.icon_square_code, 'Code block'},
	Tool{.math_block, kit.icon_sigma, 'Math block'},
	Tool{.diagram, kit.icon_workflow, 'Mermaid diagram'}, Tool{.table, kit.icon_table, 'Table'},
	Tool{.rule, kit.icon_separator_horizontal, 'Horizontal rule'}]

fn keys_of(cmd Cmd) string {
	for c in all_commands() {
		if c.cmd == cmd {
			return c.keys
		}
	}
	return ''
}

// format runs a formatting command on the shown document and keeps the editor focused.
fn (mut f Folio) format(cmd Cmd, mut cx core.ViewCx[Folio]) {
	if f.tabs.len == 0 || layout_of(f.prefs.get(cx.Cx).layout) == .preview {
		return
	}
	mut tab := f.cur()
	mut e := cx.peek(tab.editor.model())
	mut d := e.doc
	v := d.version
	format_doc(mut d, cmd)
	if d.version != v {
		tab.text = d.text()
		e.reveal = true
		cx.touch(tab.editor.model())
	}
	f.focus_editor(mut cx)
}

const layout_tools = [Tool{.raw, kit.icon_pencil, 'Raw'}, Tool{.split, kit.icon_columns_2, 'Split'},
	Tool{.preview, kit.icon_book_open, 'Preview'}]

// toolbar: where the reader is (file and section) on the left; formatting (when the editor
// is shown) and the layout switch on the right.
fn (mut f Folio) toolbar(lay Layout, t &kit.Theme, mut cx core.ViewCx[Folio]) core.Element {
	mut bar :=
		ui.div().test_id('toolbar').flex_row().items_center().gap(2).h(34).px(8).shrink_0().bg(t.p.canvas).border_b(1).border_color(t.p.line).overflow_hidden()
	bar.child(f.breadcrumb(t)).child(ui.div().grow())
	if lay != .preview {
		f.format_tools(mut bar, t, mut cx)
		bar.child(sep(t))
	}
	for k, tool in layout_tools {
		bar.child(f.tool_button(tool, mut cx).selected(int(lay) == k))
	}
	return bar
}

// breadcrumb is the file name and the headings down to the section being read.
fn (f &Folio) breadcrumb(t &kit.Theme) core.Element {
	tab := f.cur()
	mut crumbs := [tab.name()]
	mut chain := []string{}
	mut level := 7
	for k := tab.sync.section; k >= 0 && k < tab.heads.len; k-- {
		h := tab.heads[k]
		if h.level < level {
			chain.prepend(h.text)
			level = h.level
		}
	}
	crumbs << chain
	mut row :=
		ui.div().test_id('breadcrumb').flex_row().items_center().gap(6).px(4).min_w(0).overflow_hidden()
	for i, c in crumbs {
		if i > 0 {
			row.child(ui.text('›').color(t.p.text_3))
		}
		row.child(ui.text(c).color(if i == crumbs.len - 1 { t.p.text } else { t.p.text_2 }))
	}
	return row
}

fn (mut f Folio) tool_button(tool Tool, mut cx core.ViewCx[Folio]) &kit.Button {
	return kit.icon_button('fmt-${tool.cmd}', tool.glyph).ghost().size(.sm).tooltip_keys(tool.tip,
		keys_of(tool.cmd)).on_click(core.listener_with[Folio, Cmd, core.ClickEvent](cx.entity,
		tool.cmd, fn (mut f Folio, c Cmd, _ &core.ClickEvent, mut cx core.ViewCx[Folio]) {
		f.run(c, mut cx)
	}))
}

fn (mut f Folio) format_tools(mut bar ui.Div, t &kit.Theme, mut cx core.ViewCx[Folio]) {
	to := cx.listener(fn (mut f Folio, c &Cmd, mut cx core.ViewCx[Folio]) {
		f.run(*c, mut cx)
	})
	mut heads := []kit.MenuItem{}
	for c in [Cmd.para, .h1, .h2, .h3] {
		heads << kit.MenuItem{
			label:    format_commands.filter(it.cmd == c)[0].title.all_after(': ')
			shortcut: keys_of(c)
			action:   RunCmd{to, c}
		}
	}
	bar.child(kit.menu_button('fmt-heading', '', heads).icon(kit.icon_heading).size(.sm).tooltip('Heading'))
	for group in tool_groups {
		bar.child(sep(t))
		for tool in group {
			bar.child(f.tool_button(tool, mut cx))
		}
	}
	inserts := insert_tools.map(kit.MenuItem{
		label:  it.tip
		glyph:  it.glyph
		action: RunCmd{to, it.cmd}
	})
	bar.child(sep(t)).child(kit.menu_button('fmt-insert', 'Insert', inserts).icon(kit.icon_plus).size(.sm))
}

fn sep(t &kit.Theme) &ui.Div {
	return ui.div().w(1).h(16).mx(4).shrink_0().bg(t.p.line)
}
