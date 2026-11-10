// The suggestion engine: a pure function of its inputs. Parses the line, resolves the command
// tree, gathers candidates from the enabled sources, then matches, ranks and dedups.
module completion

// SuggestionKind drives the tag badge.
pub enum SuggestionKind {
	history
	command
	option
}

// tag is the single-letter badge.
pub fn (k SuggestionKind) tag() string {
	return match k {
		.history { 'H' }
		.command { 'C' }
		.option { 'O' }
	}
}

// Suggestion is one candidate returned to the UI.
pub struct Suggestion {
pub:
	text         string // the completion target
	kind         SuggestionKind
	description  string // hint shown after the text, '' for none; only options carry one
	match_len    int    // matched-prefix bytes, from the start of `text`, for highlight
	score        f32
	replace_from int // byte offset where the replaced text begins: the token, or 0 for whole-line history
}

// is_prefix_of_typed tells whether `typed` is a case-insensitive prefix of the suggestion.
pub fn (s Suggestion) is_prefix_of_typed(typed string) bool {
	return prefix_match(s.text, typed, true)
}

// Context is the query the UI hands the engine each keystroke.
pub struct Context {
pub:
	family     ShellFamily
	line       string // prompt-relative
	cursor_col int    // byte offset in `line`
	now_ms     u64    // for frecency; the engine stays clock-free
}

// Resolved is the position in the command tree.
struct Resolved {
	active           &CommandNode
	ancestor_options []Flag
	path             []string // names from the root to `active`
}

// Engine holds the lazily-parsed embedded catalogs.
pub struct Engine {
mut:
	catalog Catalog
}

// new_engine builds the engine from the embedded catalogs.
pub fn new_engine() Engine {
	return Engine{
		catalog: catalog_from(catalog_files)
	}
}

// Query bundles the inputs of the gather helpers.
struct Query {
	history &History
	ctx     Context
	cfg     Params
	family  ShellFamily
}

// Candidate is a suggestion with its ranking metadata.
struct Candidate {
mut:
	text         string
	kind         SuggestionKind
	description  string
	is_manual    bool
	is_prefix    bool
	match_len    int
	frecency     f32
	replace_from int
}

// suggest returns a ranked, deduped, redaction-safe suggestion list.
pub fn (mut e Engine) suggest(history &History, ctx Context, cfg Params) []Suggestion {
	family := ctx.family
	cursor := if ctx.cursor_col > ctx.line.len { ctx.line.len } else { ctx.cursor_col }
	p := parse_line(ctx.line, cursor)
	mut out := []Candidate{}
	q := Query{unsafe { history }, ctx, cfg, family}
	token := p.token
	is_option := token.len > 0 && family.is_option_token(token)

	if is_option && p.head != none {
		e.gather_options(q, p, token, mut out)
	} else if p.is_first_token {
		if token.len < cfg.min_prefix_len && !(cfg.suggest_on_empty && token.len == 0) {
			return []
		}
		e.gather_commands(q, p, token, mut out)
	} else {
		if r := e.resolve(p, family, cfg.windows_allow_coreutils) {
			for child in r.active.subcommands {
				if m := match_token(child.name, token, family.case_insensitive(), cfg) {
					out << Candidate{
						text:         child.name
						kind:         .command
						is_manual:    true
						is_prefix:    m.is_prefix
						match_len:    m.len
						replace_from: p.token_start
					}
				}
			}
		}
		e.gather_history_whole_line(q, mut out)
	}
	return e.finalize(mut out, family, cfg)
}

// breadcrumb returns the names from the root to the active node of the line left of the
// cursor, [] for an unknown command.
pub fn (mut e Engine) breadcrumb(ctx Context, cfg Params) []string {
	cursor := if ctx.cursor_col > ctx.line.len { ctx.line.len } else { ctx.cursor_col }
	r := e.resolve(parse_line(ctx.line, cursor), ctx.family, cfg.windows_allow_coreutils) or {
		return []
	}
	return r.path
}

// resolve walks the tokens left of the cursor to the active node, none for an unknown command.
fn (mut e Engine) resolve(p ParsedLine, family ShellFamily, allow_coreutils bool) ?Resolved {
	categories := family.categories(allow_coreutils)
	head := p.head or { return none }
	mut active := e.catalog.lookup(head, categories, family)?
	mut ancestor_options := []Flag{}
	mut path := [active.name]
	for tok in p.prior_tokens[1..] {
		if family.is_option_token(tok) {
			continue
		}
		child := active.child(tok, family) or { break }
		ancestor_options << active.options
		active = unsafe { child }
		path << active.name
	}
	return Resolved{active, ancestor_options, path}
}
