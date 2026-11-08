// Mermaid diagrams drawn natively: `mermaid.diagram(src)` parses the body of a ```mermaid
// fence once and is an element sized to the laid-out diagram. Flowcharts and sequence
// diagrams are supported (specs/21); other kinds and parse errors render nothing and report
// themselves through error(), so the caller can show the source instead.
module mermaid

import quadra.core
import quadra.prim
import quadra.kit
import quadra.layout
import quadra.text

@[heap]
pub struct Diagram {
mut:
	kind    string
	err     ?string
	flow    Flow
	seq     Seq
	drawing Drawing
	key     string // the font the drawing was laid out with
	node    layout.NodeId
}

pub fn diagram(src string) &Diagram {
	mut d := &Diagram{}
	lines := src.replace('\r\n', '\n').split('\n')
	mut i := 0
	// Skip blank lines, comments and a front matter block (`---` ... `---`).
	for i < lines.len {
		t := lines[i].trim_space()
		if t == '---' {
			i++
			for i < lines.len && lines[i].trim_space() != '---' {
				i++
			}
		} else if t != '' && !t.starts_with('%%') {
			break
		}
		i++
	}
	if i >= lines.len {
		d.err = 'empty diagram'
		return d
	}
	words := lines[i].trim_space().trim_right(';').fields()
	d.kind = words[0]
	match words[0] {
		'graph', 'flowchart' {
			d.kind = 'flowchart'
			dir := if words.len > 1 { words[1].to_upper() } else { 'TD' }
			d.flow = parse_flow(dir, lines[i + 1..], i + 1) or {
				d.err = err.msg()
				Flow{}
			}
		}
		'sequenceDiagram' {
			d.kind = 'sequence'
			d.seq = parse_seq(lines[i + 1..], i + 1) or {
				d.err = err.msg()
				Seq{}
			}
		}
		else {
			d.err = 'diagram kind `${words[0]}` is not supported'
		}
	}

	return d
}

// error is why the diagram cannot be drawn: a parse error with its line, or an
// unsupported kind.
pub fn (d &Diagram) error() ?string {
	return d.err
}

// kind is 'flowchart', 'sequence', or the keyword of an unsupported kind.
pub fn (d &Diagram) kind() string {
	return d.kind
}

// size is the laid-out size (zero before the first layout or on error).
pub fn (d &Diagram) size() prim.Size {
	return prim.Size{d.drawing.width, d.drawing.height}
}

fn (mut d Diagram) lay_out(mut ts text.TextSystem, st text.TextStyle) {
	key := '${st.family}:${st.size}'
	if key == d.key {
		return
	}
	d.key = key
	mut m := Measure{
		ts:   ts
		base: st
	}
	d.drawing = if d.kind == 'flowchart' {
		layout_flow(&d.flow, mut m)
	} else {
		layout_seq(&d.seq, mut m)
	}
}

pub fn (mut d Diagram) request_layout(mut cx core.LayoutCx) layout.NodeId {
	mut w, mut h := f32(0), f32(0)
	if d.err == none {
		t := kit.theme_of(cx.app)
		d.lay_out(mut cx.text, text.TextStyle{
			family: t.font_family
			size:   t.font_size(.md)
		})
		w, h = d.drawing.width, d.drawing.height
	}
	d.node = cx.tree.node(layout.Style{
		width:  layout.px(w)
		height: layout.px(h)
	}, [])
	return d.node
}

pub fn (mut d Diagram) prepaint(_ prim.Rect, mut _ core.PrepaintCx) {}

pub fn (mut d Diagram) paint(bounds prim.Rect, mut cx core.PaintCx) {
	if d.err != none {
		return
	}
	paint_drawing(&d.drawing, prim.pt(bounds.x, bounds.y), kit.theme_of(cx.app), mut cx)
}
