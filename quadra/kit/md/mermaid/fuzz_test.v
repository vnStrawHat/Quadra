// Fuzz the Mermaid parsers and layouts (specs/11, fuzz tests): diagrams come from untrusted
// Markdown, so any source must parse and lay out without a panic, a hang or unbounded memory.
module mermaid

import strings
import quadra.text
import quadra.text.fake
import quadra.testing.fuzz

const mm_corpus = [
	'graph TD\nA[Start] --> B{Is it?}\nB -->|Yes| C[OK]\nC --> D[Rethink]\nD --> B\nB ---->|No| E[End]\nsubgraph work [Work]\nC\nD\nend\nF((Hub)) --> A & C & E',
	'flowchart LR\nA --> B\nB --- C\nC -.-> D\nD ==> E\nE --o F\nF --x G\nG <--> H\nH --->|long| I\nI -- text --> J\nJ -. dots .-> K',
	'graph TD\na[Rect]\nb(Round)\nc([Stadium])\nd((Circle))\ne{Diamond}\nf{{Hex}}\ng[[Sub]]\nh[(DB)]\ni>Flag]\nj["Quoted [x]"]\nk:::cls\nclassDef cls fill:#f00\n%% comment',
	'---\ntitle: x\n---\nflowchart BT\nsubgraph a\nsubgraph b [B]\nx --> y\nend\nend\ny --> x; x --> x',
	'sequenceDiagram\nparticipant A as Alice\nactor B\nA->>B: Hello\nB-->>A: Hi\nA-)B: async\nNote right of A: thinking\nNote over A,B: both\nloop Every minute\nA->>B: ping\nend\nalt ok\nB->>A: yes\nelse no\nB->>A: no\nend',
	'sequenceDiagram\nautonumber\nA->>+B: open\nB-->>-A: close\nA-xB: lost\nopt maybe\nA->>A: self\nend\nrect rgb(0,0,0)\nA->>B: x\nend\npar a\nA->>B: 1\nand b\nB->>A: 2\nend',
	'graph\nA-->B-->C-->A\nA & B & C --> D & E & F\n漢字 --> 😀["Xin chào"]',
]

fn deep(mut r fuzz.Rng) string {
	n := 50 + r.intn(600)
	match r.intn(6) {
		0 {
			return 'graph TD\n' + 'subgraph s\n'.repeat(n) + 'a --> b\n' + 'end\n'.repeat(r.intn(n))
		}
		1 {
			return 'graph LR\na ' + strings.repeat(`-`, n) + '> b'
		}
		2 { // many nodes in a chain or a cycle
			mut s := ['flowchart TD']
			for k in 0 .. n {
				s << 'n${k} --> n${(k + 1 + r.intn(3)) % n}'
			}
			return s.join('\n')
		}
		3 { // wide `&` groups: every pair is an edge
			mut ids := []string{}
			for k in 0 .. 1 + n / 10 {
				ids << 'v${k}'
			}
			g := ids.join(' & ')
			return 'graph TD\n${g} --> ${g}'
		}
		4 {
			return 'sequenceDiagram\n' + 'loop l\n'.repeat(n) + 'A->>B: x\n' +
				'end\n'.repeat(r.intn(n))
		}
		else {
			mut s := ['sequenceDiagram']
			for k in 0 .. n {
				s << 'P${k % 97}->>P${r.intn(97)}: m${k}'
			}
			return s.join('\n')
		}
	}
}

fn check_diagram(src string, mut m Measure) {
	d := diagram(src)
	if d.err != none {
		return
	}
	_ = if d.kind == 'flowchart' { layout_flow(&d.flow, mut m) } else { layout_seq(&d.seq, mut m) }
	m.ts.end_frame()
}

fn test_fuzz_mermaid() {
	mut m := Measure{
		ts:   text.new_system(fake.new())
		base: text.TextStyle{
			size: 14
		}
	}
	corpus := mm_corpus.map(it.bytes())
	for c in mm_corpus {
		assert diagram(c).err == none, c
		check_diagram(c, mut m)
	}
	// Regression: a header line of only `;` has no words (it indexed an empty list).
	assert diagram(';').err != none
	// Regressions: sizes past the caps (limits.v) are errors before any layout runs; they made
	// the layout allocate gigabytes (every pair of two wide `&` groups, a cycle's dummy nodes).
	mut wide := []string{}
	for k in 0 .. 100 {
		wide << 'v${k}'
	}
	g := wide.join(' & ')
	assert diagram('graph TD\n${g} --> ${g}').err != none // 10 000 links
	assert diagram('graph TD\n' + 'subgraph s\n'.repeat(200) + 'a').err != none
	mut cyc := ['graph TD']
	for k in 1 .. 600 {
		cyc << 'n${k - 1} --> n${k}'
		cyc << 'n${k} --> n0' // back edges spanning the whole chain
	}
	assert diagram(cyc.join('\n')).err != none
	long := diagram('graph LR\na ' + strings.repeat(`-`, 100_000) + '> b')
	assert long.err == none && long.flow.edges[0].minlen == max_link_len
	assert diagram('sequenceDiagram\n' + 'A->>B: x\n'.repeat(max_seq_items + 1)).err != none
	// A node repeated in an `&` group is one node: 500 copies made 1500 parallel links and a
	// 2 s layout (found by the long run).
	rep := diagram('graph\nA & ' + 'B& '.repeat(500) + 'C --> D & E & F')
	assert rep.err == none && rep.flow.edges.len == 9
	n := fuzz.iters(1000)
	mut r := fuzz.new_rng(0x6d6d)
	mut w := fuzz.watch('mermaid', n)
	for i in 0 .. n {
		mut src := ''
		if r.chance(8) {
			src = deep(mut r)
		} else {
			src = r.mutate(corpus[r.intn(corpus.len)], corpus).bytestr()
		}
		w.start()
		check_diagram(src, mut m)
		w.done(i, src.len, 2000)
	}
	w.finish(64)
}
