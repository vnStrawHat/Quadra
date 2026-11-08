// Scroll sync and the current section. Sync is an empty element placed before the preview's
// scroll area: at every prepaint it moves the preview to the editor's top line (Split, one
// way) and finds the section at the top of the view for the outline.
module main

import quadra.core
import quadra.prim
import quadra.kit.code
import quadra.kit.md
import quadra.layout
import quadra.ui

@[heap]
pub struct SyncState {
pub mut:
	editor_top f32 = -1 // the editor's top line at the last sync
	section    int = -1 // the outline entry of the section at the top
}

// reading_line: the current section is the one at this depth into the view, not at its very
// top: a heading just below the top (or under front matter) is what the reader is on.
const reading_line = f32(120)

struct Sync {
	owner    core.View[Folio] // re-rendered when the section changes (the outline shows it)
	editor   core.View[code.Editor]
	pos      &md.Positions   = unsafe { nil }
	scroll   &ui.ScrollState = unsafe { nil }
	st       &SyncState      = unsafe { nil }
	follow   bool  // Split: the preview follows the editor
	preview  bool  // the preview is shown (else the editor gives the top line)
	headings []int // the source lines of the outline's entries
mut:
	node layout.NodeId
}

fn (mut s Sync) request_layout(mut cx core.LayoutCx) layout.NodeId {
	s.node = cx.tree.node(layout.Style{}, [])
	return s.node
}

fn (mut s Sync) prepaint(_ prim.Rect, mut cx core.PrepaintCx) {
	mut c := core.Cx{
		app: cx.app
	}
	mut ed := c.peek(s.editor.model())
	mut st := s.st
	mut scroll := s.scroll
	editor_top := ed.top_line()
	if s.follow && editor_top != st.editor_top {
		st.editor_top = editor_top
		scroll.y = if editor_top <= 0 { f32(0) } else { s.pos.y_of(editor_top) + preview_pad }
	}
	top := if s.preview {
		s.pos.line_at(scroll.y - preview_pad + reading_line)
	} else {
		int(editor_top) + 5
	}
	mut section := -1
	for k, line in s.headings {
		if line > top {
			break
		}
		section = k
	}
	if section != st.section {
		st.section = section
		c.touch(s.owner.model()) // the outline shows it: the next frame renders again
	}
}

fn (mut s Sync) paint(_ prim.Rect, mut _ core.PaintCx) {}
