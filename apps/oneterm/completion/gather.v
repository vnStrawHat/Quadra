// Per-context candidate gathering, matching and final ranking.
module completion

// Weights of the ranking blend; fixed, never user-exposed.
const w_kind = f32(10.0)
const w_frec = f32(6.0)
const w_prefix = f32(4.0)
const w_len = f32(1.0)

struct Match {
	is_prefix bool
	len       int
}

// match_token matches a name against a token: prefix first, then fuzzy.
fn match_token(name string, token string, ci bool, cfg Params) ?Match {
	if token.len == 0 {
		return Match{true, 0}
	}
	if prefix_match(name, token, ci) {
		return Match{true, token.len}
	}
	if cfg.fuzzy && is_subsequence(name, token, ci) {
		return Match{false, 0}
	}
	return none
}

// match_option matches an option flag, case-insensitively (Windows `/Q` vs `/q`).
fn match_option(flag string, token string, cfg Params) ?Match {
	if token.len == 0 {
		return Match{true, 0}
	}
	if prefix_match(flag, token, true) {
		return Match{true, token.len}
	}
	if cfg.fuzzy && is_subsequence(flag, token, true) {
		return Match{false, 0}
	}
	return none
}

// is_subsequence: an ASCII needle compares bytewise, anything else by runes.
fn is_subsequence(haystack string, needle string, ci bool) bool {
	if needle.len == 0 {
		return true
	}
	if is_ascii(needle) {
		mut n := 0
		for c in haystack {
			if n < needle.len
				&& (c == needle[n] || (ci && lower_ascii(c) == lower_ascii(needle[n]))) {
				n++
			}
		}
		return n == needle.len
	}
	hay := if ci { lower_runes(haystack.runes()) } else { haystack.runes() }
	need := if ci { lower_runes(needle.runes()) } else { needle.runes() }
	mut n := 0
	for c in hay {
		if n < need.len && c == need[n] {
			n++
		}
	}
	return n == need.len
}

fn lower_runes(rs []rune) []rune {
	mut out := []rune{len: rs.len}
	for i, r in rs {
		out[i] = if r < 128 { rune(lower_ascii(u8(r))) } else { r }
	}
	return out
}

fn (mut e Engine) gather_commands(q Query, p ParsedLine, token string, mut out []Candidate) {
	cfg, family := q.cfg, q.family
	ci := family.case_insensitive()
	if cfg.sources.external || cfg.sources.manual {
		categories := family.categories(cfg.windows_allow_coreutils)
		for name in e.catalog.command_names(categories, family) {
			if m := match_token(name, token, ci, cfg) {
				out << Candidate{
					text:         name
					kind:         .command
					is_manual:    e.catalog.is_manual(name, categories, family)
					is_prefix:    m.is_prefix
					match_len:    m.len
					replace_from: p.token_start
				}
			}
		}
	}
	if cfg.sources.memory {
		for rec in q.history.entries(family) {
			first := first_field(rec.line)
			if first.len == 0 {
				continue
			}
			if m := match_token(first, token, ci, cfg) {
				out << Candidate{
					text:         first
					kind:         .history
					is_prefix:    m.is_prefix
					match_len:    m.len
					frecency:     rec.frecency(q.ctx.now_ms)
					replace_from: p.token_start
				}
			}
		}
	}
}

fn (mut e Engine) gather_options(q Query, p ParsedLine, token string, mut out []Candidate) {
	cfg, family := q.cfg, q.family
	if r := e.resolve(p, family, cfg.windows_allow_coreutils) {
		if cfg.sources.manual || cfg.sources.external {
			for flag in r.active.options {
				append_option(flag, token, cfg, p, 0.0, mut out)
			}
			if cfg.inherit_ancestor_options {
				for flag in r.ancestor_options {
					append_option(flag, token, cfg, p, -1.0, mut out)
				}
			}
		}
	} else if cfg.sources.memory {
		// Unknown command: options from the history of that command.
		head := p.head or { return }
		mut seen := map[string]bool{}
		for rec in q.history.entries(family) {
			toks := split_ws(rec.line)
			if toks.len == 0 || !names_eq(toks[0], head, family) {
				continue
			}
			for t in toks[1..] {
				if family.is_option_token(t) && t !in seen {
					if m := match_option(t, token, cfg) {
						seen[t] = true
						out << Candidate{
							text:         t
							kind:         .history
							is_prefix:    m.is_prefix
							match_len:    m.len
							frecency:     rec.frecency(q.ctx.now_ms)
							replace_from: p.token_start
						}
					}
				}
			}
		}
	}
}

fn append_option(flag Flag, token string, cfg Params, p ParsedLine, frecency f32, mut out []Candidate) {
	if m := match_option(flag.text, token, cfg) {
		out << Candidate{
			text:         flag.text
			kind:         .option
			description:  flag.description
			is_manual:    true
			is_prefix:    m.is_prefix
			match_len:    m.len
			frecency:     frecency
			replace_from: p.token_start
		}
	}
}

fn (mut e Engine) gather_history_whole_line(q Query, mut out []Candidate) {
	cfg, family, ctx := q.cfg, q.family, q.ctx
	if !cfg.sources.memory {
		return
	}
	ci := family.case_insensitive()
	cursor := if ctx.cursor_col > ctx.line.len { ctx.line.len } else { ctx.cursor_col }
	line_prefix := ctx.line[..cursor].trim_left(' \t\n\r\v\f')
	for rec in q.history.entries(family) {
		if rec.line.len <= line_prefix.len {
			continue // no remainder to add
		}
		if prefix_match(rec.line, line_prefix, ci) {
			out << Candidate{
				text:      rec.line
				kind:      .history
				is_prefix: true
				match_len: line_prefix.len
				frecency:  rec.frecency(ctx.now_ms)
			}
		}
	}
}

// precedence for keeping the tag on dedup: history > manual > external.
fn precedence(c Candidate) int {
	if c.kind == .history {
		return 3
	}
	return if c.is_manual { 2 } else { 1 }
}

fn score(c Candidate) f32 {
	mut kind_weight := f32(1.0)
	if c.kind == .history {
		kind_weight = 3.0
	} else if c.is_manual {
		kind_weight = 2.0
	}
	prefix_bonus := if c.is_prefix { f32(1.0) } else { f32(0.0) }
	short_bonus := f32(1.0) / (f32(1.0) + f32(c.text.len))
	return w_kind * kind_weight + w_frec * c.frecency + w_prefix * prefix_bonus +
		w_len * short_bonus
}

struct Scored {
	seq int
	val f32
mut:
	c Candidate
}

// finalize scores, drops secrets, sorts (stable), dedups and truncates.
fn (mut e Engine) finalize(mut candidates []Candidate, family ShellFamily, cfg Params) []Suggestion {
	ci := family.case_insensitive()
	mut scored := []Scored{cap: candidates.len}
	for c in candidates {
		// Suggestion-time guard, even if a raw secret reached the ring.
		if cfg.redact_sensitive && c.kind == .history && contains_secret(c.text) {
			continue
		}
		scored << Scored{scored.len, score(c), c}
	}
	// V's sort is unstable: break ties on the original position.
	scored.sort_with_compare(fn (a &Scored, b &Scored) int {
		if a.val != b.val {
			return if a.val > b.val { -1 } else { 1 }
		}
		return a.seq - b.seq
	})
	mut result := []Scored{}
	for s in scored {
		mut found := -1
		for i, existing in result {
			if (ci && eq_fold(existing.c.text, s.c.text)) || (!ci && existing.c.text == s.c.text) {
				found = i
				break
			}
		}
		if found >= 0 {
			if precedence(s.c) > precedence(result[found].c) {
				result[found].c.kind = s.c.kind
				result[found].c.is_manual = s.c.is_manual
				result[found].c.description = s.c.description
			}
			continue
		}
		result << s
	}
	if result.len > cfg.hard_cap() {
		result = unsafe { result[..cfg.hard_cap()] }
	}
	mut out := []Suggestion{cap: result.len}
	for s in result {
		out << Suggestion{
			text:         s.c.text
			kind:         s.c.kind
			description:  s.c.description
			match_len:    s.c.match_len
			score:        s.val
			replace_from: s.c.replace_from
		}
	}
	return out
}
