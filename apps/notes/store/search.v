// Full-text search: notes are folded (lower case, no diacritics) once into an index, and a
// query matches when every word appears. Run build and query in background tasks.
module store

import quadra.text

pub struct Index {
pub:
	ids   []string
	texts []string
}

pub fn fold(s string) string {
	return text.fold_diacritics(s).to_lower()
}

pub fn build_index(notes []Note) Index {
	return Index{
		ids:   notes.map(it.id)
		texts: notes.map(fold(it.body))
	}
}

// query returns the ids of matching notes, in index order. `cancelled` is polled so a
// newer query can stop an older one early.
pub fn (ix &Index) query(q string, cancelled fn () bool) []string {
	words := fold(q).fields()
	if words.len == 0 {
		return ix.ids.clone()
	}
	mut out := []string{}
	for i, t in ix.texts {
		if i % 512 == 0 && cancelled() {
			return []string{}
		}
		if words.all(t.contains(it)) {
			out << ix.ids[i]
		}
	}
	return out
}
