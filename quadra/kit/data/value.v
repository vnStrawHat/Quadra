// Value: a JSON or YAML value as a tree, for the value viewer (specs/19).
module data

import x.json2

pub enum ValueKind {
	null
	boolean
	number
	str
	list
	map
}

pub struct Value {
pub:
	kind  ValueKind
	text  string   // scalars as written (numbers keep their spelling)
	items []Value  // list items, or map values
	keys  []string // map keys, in order
}

pub fn scalar(kind ValueKind, text string) Value {
	return Value{
		kind: kind
		text: text
	}
}

// get returns the value at a map key.
pub fn (v &Value) get(key string) ?Value {
	for i, k in v.keys {
		if k == key {
			return v.items[i]
		}
	}
	return none
}

// at follows a path such as .spec.containers[0].image.
pub fn (v &Value) at(path string) ?Value {
	mut cur := *v
	mut i := 0
	for i < path.len {
		if path[i] == `.` {
			mut j := i + 1
			for j < path.len && path[j] != `.` && path[j] != `[` {
				j++
			}
			cur = cur.get(path[i + 1..j])?
			i = j
		} else if path[i] == `[` {
			j := path.index_after(']', i) or { return none }
			n := path[i + 1..j].int()
			if cur.kind != .list || n >= cur.items.len {
				return none
			}
			cur = cur.items[n]
			i = j + 1
		} else {
			return none
		}
	}
	return cur
}

// summary is the one-line form of a value: the scalar, or {n} / [n] for containers.
pub fn (v &Value) summary() string {
	return match v.kind {
		.map { '{${v.keys.len}}' }
		.list { '[${v.items.len}]' }
		.str { '"${v.text}"' }
		else { v.text }
	}
}

pub fn parse_json(s string) !Value {
	return from_any(json2.decode[json2.Any](s)!)
}

fn from_any(a json2.Any) Value {
	return match a {
		map[string]json2.Any {
			mut keys := []string{}
			mut items := []Value{}
			for k, x in a {
				keys << k
				items << from_any(x)
			}
			Value{
				kind:  .map
				keys:  keys
				items: items
			}
		}
		[]json2.Any {
			Value{
				kind:  .list
				items: a.map(from_any(it))
			}
		}
		string {
			scalar(.str, a)
		}
		bool {
			scalar(.boolean, a.str())
		}
		json2.Null {
			scalar(.null, 'null')
		}
		else {
			scalar(.number, a.str())
		}
	}
}
