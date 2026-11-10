// Grouping, search, labels and colours of the saved sessions; the panel renders what these return.
module store

pub const session_id_prefix = 'session:'
pub const group_id_prefix = 'group:'

// A blank group name is no group.
pub fn normalized_group(group ?string) ?string {
	name := group or { return none }.trim_space()
	return if name == '' { none } else { name }
}

fn rename_group_in(mut entries []Entry, old_name string, new_name string) bool {
	new_group := normalized_group(new_name)
	mut changed := false
	for mut e in entries {
		if (e.session.group or { continue }) == old_name {
			e.session.group = new_group
			changed = true
		}
	}
	return changed
}

// copy_label is `<label> (copy)`, then `(copy 2)`, `(copy 3)`, for the first candidate unused.
fn copy_label(label string, entries []Entry) string {
	mut candidate := '${label} (copy)'
	mut suffix := 2
	for entries.any(it.session.label == candidate) {
		candidate = '${label} (copy ${suffix})'
		suffix++
	}
	return candidate
}

fn duplicate_in(mut entries []Entry, id u64, new_id u64) bool {
	mut index := -1
	for i, e in entries {
		if e.id == id {
			index = i
			break
		}
	}
	if index < 0 {
		return false
	}
	mut session := entries[index].session
	session.label = copy_label(session.label, entries)
	entries.insert(index + 1, Entry{
		id:      new_id
		session: session
	})
	return true
}

fn set_group_in(mut entries []Entry, id u64, group ?string) bool {
	target := normalized_group(group)
	for mut e in entries {
		if e.id != id {
			continue
		}
		if (e.session.group or { '\0' }) == target or { '\0' } {
			return false
		}
		e.session.group = target
		return true
	}
	return false
}

fn set_sealed_in(mut entries []Entry, id u64, host string, port int, username string, sealed string) ! {
	for mut e in entries {
		if e.id == id {
			if !e.session.has_address(host, port, username) {
				return error("the session's address or method changed meanwhile.")
			}
			e.session.sealed_password = sealed
			return
		}
	}
	return error('the session no longer exists.')
}

fn clear_sealed_in(mut entries []Entry) bool {
	mut changed := false
	for mut e in entries {
		if e.session.sealed_password != none {
			e.session.sealed_password = none
			changed = true
		}
	}
	return changed
}

// session_subtitle is `user@host:port`, or `host:port` without a user.
pub fn session_subtitle(s Session) string {
	if username := s.username {
		return '${username}@${s.host}:${s.port}'
	}
	return '${s.host}:${s.port}'
}

fn is_hex_color(text string) bool {
	digits := text.trim_string_left('#')
	return digits.len in [6, 8] && digits.contains_only('0123456789abcdefABCDEF')
}

// session_color_hex is the colour every surface draws a session with: the saved one when it is
// a valid 6 or 8 digit colour (a leading # is optional), else the default.
pub fn session_color_hex(s Session) string {
	color := s.color or { '' }
	return if is_hex_color(color) { color } else { default_color_hex }
}

// session_matches: `query` must already be trimmed and lowercased.
pub fn session_matches(s Session, query string) bool {
	return s.label.to_lower().contains(query) || s.host.to_lower().contains(query) || (s.username or {
		''
	}).to_lower().contains(query) || (s.group or { '' }).trim_space().to_lower().contains(query)
}

// TreeItem is one row of the session tree; a group has children.
pub struct TreeItem {
pub:
	id       string
	label    string
	children []TreeItem
}

fn leaf(e Entry) TreeItem {
	return TreeItem{
		id:    '${session_id_prefix}${e.id}'
		label: e.session.label
	}
}

fn by_label(a &Entry, b &Entry) int {
	x, y := a.session.label.to_lower(), b.session.label.to_lower()
	return if x < y {
		-1
	} else if x > y {
		1
	} else {
		0
	}
}

// build_tree filters by `query` (empty shows everything), then lists ungrouped sessions by
// label, then one group per name, sorted, with its members by label.
pub fn build_tree(sessions []Entry, query string) []TreeItem {
	q := query.trim_space().to_lower()
	mut ungrouped := []Entry{}
	mut groups := map[string][]Entry{}
	for e in sessions {
		if q != '' && !session_matches(e.session, q) {
			continue
		}
		if group := normalized_group(e.session.group) {
			groups[group] << e
		} else {
			ungrouped << e
		}
	}
	ungrouped.sort_with_compare(by_label)
	mut items := ungrouped.map(leaf(it))
	mut names := groups.keys()
	names.sort()
	for name in names {
		mut members := groups[name].clone()
		members.sort_with_compare(by_label)
		items << TreeItem{
			id:       '${group_id_prefix}${name}'
			label:    name
			children: members.map(leaf(it))
		}
	}
	return items
}

// parse_session_item is the session id of a tree item id.
pub fn parse_session_item(id string) ?u64 {
	if !id.starts_with(session_id_prefix) {
		return none
	}
	text := id[session_id_prefix.len..]
	return if text != '' && text.contains_only('0123456789') { text.u64() } else { none }
}

// parse_group_item is the group name of a tree item id.
pub fn parse_group_item(id string) ?string {
	return if id.starts_with(group_id_prefix) { id[group_id_prefix.len..] } else { none }
}

// MenuRow is one saved session in the New Terminal menu.
pub struct MenuRow {
pub:
	id    u64
	title string
	color string
}

// MenuSection groups rows under `group`; '' is the ungrouped section.
pub struct MenuSection {
pub:
	group string
pub mut:
	rows []MenuRow
}

// menu_entries keeps storage order: ungrouped first, then each group where it first appears.
pub fn menu_entries(sessions []Entry) []MenuSection {
	mut ungrouped := []MenuRow{}
	mut groups := []MenuSection{}
	for e in sessions {
		label := e.session.label.trim_space()
		row := MenuRow{
			id:    e.id
			title: if label == '' { session_subtitle(e.session) } else { label }
			color: session_color_hex(e.session)
		}
		group := normalized_group(e.session.group) or {
			ungrouped << row
			continue
		}
		mut found := false
		for mut section in groups {
			if section.group == group {
				section.rows << row
				found = true
				break
			}
		}
		if !found {
			groups << MenuSection{
				group: group
				rows:  [row]
			}
		}
	}
	mut sections := []MenuSection{}
	if ungrouped.len > 0 {
		sections << MenuSection{
			rows: ungrouped
		}
	}
	sections << groups
	return sections
}
