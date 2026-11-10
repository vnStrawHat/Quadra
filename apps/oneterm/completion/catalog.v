// The bundled command catalog: an index over the embedded files plus a lazy parse cache.
// Lookups honor the shell's category search path and manual-beats-external precedence.
module completion

import x.json2

const supported_schema = 1

enum Source {
	external
	manual
}

// CatalogFile is one embedded file: `(name, source, category, json)` (catalog_index.v).
struct CatalogFile {
	name     string
	source   string
	category string
	json     string
}

struct Flag {
	text        string
	description string // argument placeholder or short hint, '' for none
}

// CommandNode is a recursive command: name, options, subcommands.
struct CommandNode {
	name        string
	options     []Flag
	subcommands []CommandNode
}

fn names_eq(a string, b string, family ShellFamily) bool {
	return if family.case_insensitive() { eq_fold(a, b) } else { a == b }
}

// child finds a direct child subcommand by name.
fn (n &CommandNode) child(name string, family ShellFamily) ?&CommandNode {
	for i in 0 .. n.subcommands.len {
		if names_eq(n.subcommands[i].name, name, family) {
			return unsafe { &n.subcommands[i] }
		}
	}
	return none
}

struct Entry {
	name     string
	source   Source
	category Category
	json     string
}

struct Catalog {
mut:
	entries []Entry
	state   []u8 // 0 not parsed, 1 parsed, 2 unparseable
	nodes   []&CommandNode
}

fn catalog_from(files []CatalogFile) Catalog {
	mut c := Catalog{}
	for f in files {
		src := match f.source {
			'external' { Source.external }
			'manual' { Source.manual }
			else { continue }
		}

		cat := category_from_str(f.category) or { continue }
		c.entries << Entry{f.name, src, cat, f.json}
	}
	c.state = []u8{len: c.entries.len}
	c.nodes = []&CommandNode{len: c.entries.len, init: unsafe { nil }}
	return c
}

// parse_node reads one catalog file; an unknown major schema is an error.
fn parse_node(json string) !CommandNode {
	doc := json2.decode[json2.Any](json)!
	if doc !is map[string]json2.Any {
		return error('a catalog file is a JSON object')
	}
	o := doc.as_map()
	if v := o['schema'] {
		if v.int() > supported_schema {
			return error('unsupported catalog schema version ${v.int()}')
		}
	}
	return node_from_map(o)
}

fn node_from_map(o map[string]json2.Any) !CommandNode {
	name := o['name'] or { return error('missing name') }
	mut options := []Flag{}
	if raw_options := o['options'] {
		for item in raw_options.as_array() {
			if item is map[string]json2.Any {
				flag := item['flag'] or { return error('missing flag') }
				d := item['description'] or { json2.Any('') }
				options << Flag{flag.str(), d.str().trim_space()}
			} else {
				options << Flag{item.str(), ''}
			}
		}
	}
	mut subcommands := []CommandNode{}
	if raw_subs := o['subcommands'] {
		for item in raw_subs.as_array() {
			if item is map[string]json2.Any {
				subcommands << node_from_map(item)!
			} else {
				return error('a subcommand is an object')
			}
		}
	}
	return CommandNode{name.str(), options, subcommands}
}

// node_at parses entry `idx` once; a malformed file gives nil and stays isolated.
fn (mut c Catalog) node_at(idx int) ?&CommandNode {
	match c.state[idx] {
		1 {
			return c.nodes[idx]
		}
		2 {
			return none
		}
		else {}
	}

	node := parse_node(c.entries[idx].json) or {
		c.state[idx] = 2
		return none
	}
	heap := &CommandNode{node.name, node.options, node.subcommands}
	c.nodes[idx] = heap
	c.state[idx] = 1
	return heap
}

fn rank_of(categories []Category, c Category) int {
	for i, x in categories {
		if x == c {
			return i
		}
	}
	return -1
}

// better: manual beats external; within a source the earlier category wins.
fn better(source Source, rank int, best_source Source, best_rank int) bool {
	if source == .manual && best_source == .external {
		return true
	}
	if source == .external && best_source == .manual {
		return false
	}
	return rank < best_rank
}

// best_entry returns the index of the highest-precedence entry for `name`, or -1.
fn (c &Catalog) best_entry(name string, categories []Category, family ShellFamily) int {
	mut best := -1
	mut best_rank := 0
	mut best_source := Source.external
	for i, entry in c.entries {
		if !names_eq(entry.name, name, family) {
			continue
		}
		rank := rank_of(categories, entry.category)
		if rank < 0 {
			continue
		}
		if best < 0 || better(entry.source, rank, best_source, best_rank) {
			best = i
			best_rank = rank
			best_source = entry.source
		}
	}
	return best
}

// lookup finds a top-level command node.
fn (mut c Catalog) lookup(name string, categories []Category, family ShellFamily) ?&CommandNode {
	idx := c.best_entry(name, categories, family)
	if idx < 0 {
		return none
	}
	return c.node_at(idx)
}

// command_names lists the distinct top-level names of the searched categories, keeping the
// highest-precedence spelling at the first-seen position. Never parses JSON.
fn (c &Catalog) command_names(categories []Category, family ShellFamily) []string {
	mut positions := map[string]int{}
	mut ranks := []int{}
	mut sources := []Source{}
	mut order := []string{}
	for entry in c.entries {
		rank := rank_of(categories, entry.category)
		if rank < 0 {
			continue
		}
		key := if family.case_insensitive() { entry.name.to_lower() } else { entry.name }
		if pos := positions[key] {
			if better(entry.source, rank, sources[pos], ranks[pos]) {
				ranks[pos] = rank
				sources[pos] = entry.source
				order[pos] = entry.name
			}
		} else {
			positions[key] = order.len
			ranks << rank
			sources << entry.source
			order << entry.name
		}
	}
	return order
}

// is_manual tells whether the highest-precedence entry for `name` came from `manual`.
fn (c &Catalog) is_manual(name string, categories []Category, family ShellFamily) bool {
	idx := c.best_entry(name, categories, family)
	return idx >= 0 && c.entries[idx].source == .manual
}
