// Comparison scenes (docs/benchmarks.md): the same four small apps other frameworks publish
// numbers for, with the same text, sizes and frame recording. The scene is the first
// argument; bench/compare/run starts the app and measures it from outside.
//   counter   360×260  a number and three buttons, then idle
//   list      800×600  a virtual list of 100 000 rows of three texts, scrolling 8 px a frame
//   grid      800×600  600 cells, each with its background and a number changing every frame
//   document  800×600  40 paragraphs (~20 000 chars) wrapped to a width changing every frame
module main

import math
import os
import quadra.core
import quadra.prim
import quadra.native
import quadra.platform
import quadra.ui

const words_list = ['window', 'button', 'layout', 'render', 'scroll', 'update', 'shadow', 'cursor',
	'memory', 'thread', 'signal', 'bitmap', 'stroke', 'column', 'editor', 'kernel', 'packet',
	'vector', 'syntax', 'buffer', 'format', 'cipher', 'socket', 'module', 'branch', 'commit',
	'server', 'client', 'screen', 'pointer', 'display', 'palette']

// words is the text of a seed, the same in every framework's app.
fn words(seed int, n int) string {
	mut x := u32(seed)
	mut out := []string{cap: n}
	for _ in 0 .. n {
		x = x * 1103515245 + 12345
		out << words_list[(x >> 16) % u32(words_list.len)]
	}
	return out.join(' ')
}

fn cell_value(cell int, frame int) f64 {
	return f64((cell * 7919 + frame * 31) % 100000) / 100
}

fn cell_color(v f64) prim.Color {
	return if v < 333 {
		prim.rgb(0xf6c9c4)
	} else if v < 666 {
		prim.rgb(0xf4e8b8)
	} else {
		prim.rgb(0xc6e9c8)
	}
}

fn document_width(frame int) f32 {
	return f32(math.round(600 + 150 * math.sin(f64(frame) / 30)))
}

struct Counter {
mut:
	n int
}

fn button(id string, label string) &ui.Div {
	return ui.div().id(id).px(12).py(4).rounded(6).bg(prim.rgb(0xe8e8e8)).text_size(13).child(ui.text(label))
}

@[markused]
pub fn (mut c Counter) render(mut cx core.ViewCx[Counter]) core.Element {
	return ui.div().size_full().flex_col().items_center().justify_center().gap(16).bg(prim.white).text_color(prim.black).child(ui.text(c.n.str()).size(56).weight(700)).child(ui.div().flex_row().gap(8).child(button('decrement',
		'−').on_click(cx.listener(fn (mut c Counter, _ &core.ClickEvent, mut _ core.ViewCx[Counter]) {
		c.n--
	}))).child(button('reset', 'Reset').on_click(cx.listener(fn (mut c Counter, _ &core.ClickEvent, mut _ core.ViewCx[Counter]) {
		c.n = 0
	}))).child(button('increment', '+').on_click(cx.listener(fn (mut c Counter, _ &core.ClickEvent, mut _ core.ViewCx[Counter]) {
		c.n++
	}))))
}

struct List {
mut:
	rec Recorder
	y   f32
}

fn list_row(i int) core.Element {
	return ui.div().w_full().h(40).px(12).gap(12).flex_row().items_center().child(ui.div().grow().flex_col().child(ui.text('Item ${i}').size(13)).child(ui.text(words(
		i + 1, 6)).size(11).color(prim.rgb(0x707070)))).child(ui.number(cell_value(i, 0), 2).size(13))
}

@[markused]
pub fn (mut l List) render(mut cx core.ViewCx[List]) core.Element {
	if l.rec.tick(cx.win) {
		cx.request_animation_frame()
		l.y += 8
	}
	return ui.div().size_full().bg(prim.white).text_color(prim.black).child(ui.uniform_list('rows',
		100000, 40, list_row).scroll_y(l.y))
}

struct Grid {
mut:
	rec   Recorder
	frame int
}

@[markused]
pub fn (mut g Grid) render(mut cx core.ViewCx[Grid]) core.Element {
	if g.rec.tick(cx.win) {
		cx.request_animation_frame()
		g.frame++
	}
	mut root := ui.div().size_full().flex_col().bg(prim.white)
	for row in 0 .. 30 {
		mut r := ui.div().flex_row()
		for col in 0 .. 20 {
			v := cell_value(row * 20 + col, g.frame)
			r.child(ui.div().w(40).h(20).center().bg(cell_color(v)).child(ui.number(v, 2).size(11).color(prim.rgb(0x101010))))
		}
		root.child(r)
	}
	return root
}

struct Document {
mut:
	rec        Recorder
	paragraphs []string
	frame      int
}

@[markused]
pub fn (mut d Document) render(mut cx core.ViewCx[Document]) core.Element {
	if d.rec.tick(cx.win) {
		cx.request_animation_frame()
		d.frame++
	}
	mut col := ui.div().w(document_width(d.frame)).p(12).flex_col().gap(8).text_size(13)
	for p in d.paragraphs {
		col.child(ui.div().child(ui.text(p)))
	}
	return ui.div().id('document').size_full().overflow_y_scroll().bg(prim.white).text_color(prim.black).child(col)
}

fn main() {
	scene := if os.args.len > 1 { os.args[1] } else { 'counter' }
	mut app := core.new_app(native.backends()!, core.AppOptions{})!
	w, h := if scene == 'counter' { f32(360), f32(260) } else { f32(800), f32(600) }
	opts := platform.WindowOptions{
		title:  'Benchmark'
		width:  w
		height: h
	}
	match scene {
		'counter' {
			app.open_window(opts, Counter{})!
			app.present_hooks << fn (mut _ core.App) {
				say('ready')
			}
			if os.getenv('BENCH_IDLE_PROBE') != '' {
				probe(mut app)
			}
		}
		'list' {
			app.open_window(opts, List{ rec: new_recorder() })!
		}
		'grid' {
			app.open_window(opts, Grid{ rec: new_recorder() })!
		}
		'document' {
			app.open_window(opts, Document{
				rec:        new_recorder()
				paragraphs: []string{len: 40, init: words(index + 1, 70)}
			})!
		}
		else {
			eprintln('no scene ${scene}')
			exit(2)
		}
	}

	app.run()
}

// probe prints the frames drawn and the GC collections every second (idle diagnostics).
fn probe(mut app core.App) {
	app.after(1000, fn (mut app core.App) {
		say('frames ${app.windows[0].stats.frames} builds ${app.windows[0].stats.builds} gc ${gc_heap_usage().heap_size / 1024} KB')
		probe(mut app)
	})
}
