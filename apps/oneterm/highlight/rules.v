// The rule set: the keyword trie and the compiled prompt and structural patterns, built once.
module highlight

// Keyword vocabulary, class by class; earlier lists win on a tie.
const debug_words = ['debug', 'trace', 'verbose']
const warn_words = ['closed', 'disconnected', 'exited', 'skipped', 'stopped', 'sudo', 'terminated',
	'warning', 'warn']
const info_words = ['access', 'authentication', 'connection', 'disconnection', 'info', 'login',
	'operation', 'password', 'permission']
const success_words = ['can', 'correct', 'correctly', 'known', 'ok', 'pass', 'passed', 'success',
	'successful', 'successfully', 'supported', 'true', 'valid', 'yes']
const error_words = ['bad', 'cannot', 'denied', 'deprecated', 'disabled', 'error', 'errors', 'fail',
	'failed', 'failure', 'false', 'important', 'incorrect', 'invalid', 'none', 'refused', 'unknown',
	'unsupported', 'not supported', 'wrong']

// The patterns of the original, verbatim. Literal characters are written as in Rust.
const unix_prompt_pattern = r'^(?:\([^)\s]*\) )?(?:\[[^\]]*\]|[^\s]*[@:~/\]][^\s]*)?[\$#%](?: |$)'
const win_path_body = r'[^<>|"*?:\r\n]*[^\s<>|"*?:-]'
const win_path_root = r'(?:[A-Za-z]:|\\\\)'
const pwsh_path_root = r'(?:[A-Za-z][A-Za-z0-9.-]*(?:\\[A-Za-z][A-Za-z0-9.-]*)*::)?(?:[A-Za-z]+:|\\\\|/)'
const ipv6_pattern = r'(?:(?:[0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}(?::[0-9a-fA-F]{1,4})*|::[0-9a-fA-F]{1,4}(?::[0-9a-fA-F]{1,4})*|(?:[0-9a-fA-F]{1,4}:){1,6}:|(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4})'
const mac_pattern = r'\b[0-9a-fA-F]{2}(?::[0-9a-fA-F]{2}){5}\b'
const datetime_pattern = r'\b(?:\d{4}[-/]\d{2}[-/]\d{2}(?:[ T]\d{1,2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:\s*(?:Z|UTC|GMT|AM|PM|[+-]\d{2}:?\d{2}))?)?|\d{2}/\d{2}/\d{4}(?:[ T]\d{1,2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:\s*(?:Z|UTC|GMT|AM|PM|[+-]\d{2}:?\d{2}))?)?|\d{1,2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:\s*(?:AM|PM|UTC|GMT|Z|[+-]\d{2}:?\d{2}))?|(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun)[a-z]*\s+(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\s+\d{1,2}\s+\d{1,2}:\d{2}(?::\d{2})?(?:\s*(?:AM|PM))?(?:\s+(?:UTC|GMT|[+-]\d{2}:?\d{2}))?\s+\d{4}|(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*|(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun)[a-z]*)\b'

fn win_path_prompt_pattern() string {
	return '^(?:${win_path_root}${win_path_body}>[ ]?)'
}

fn cmd_prompt_pattern() string {
	return win_path_prompt_pattern() + '|(?:^>[ ]?)'
}

fn pwsh_prompt_pattern() string {
	return '^(?:PS(?: ${pwsh_path_root}(?:${win_path_body})?)?>[ ]?)|(?:^>+[ ]?)'
}

const dumb_prompt_pattern = r'^[^\s]*[\$#%>»](?: |$)'

const trie_alphabet = 27 // a-z and space

// Rules is built once; see `global_rules`.
struct Rules {
mut:
	trie_next   []i16 // nodes * trie_alphabet, 0 = none
	trie_class  []u8  // class per node, 0 = no keyword ends here
	ipv6        Re
	mac         Re
	datetime    Re
	prompt_unix Re
	prompt_cmd  Re
	prompt_pwsh Re
	prompt_dumb Re
	prompt_any  Re // the universal fallback: Windows path or Unix prompt
}

const global_rules = build_rules()

fn build_rules() Rules {
	mut r := Rules{
		trie_next:   []i16{len: trie_alphabet}
		trie_class:  [u8(0)]
		ipv6:        re_compile(ipv6_pattern)
		mac:         re_compile(mac_pattern)
		datetime:    re_compile(datetime_pattern)
		prompt_unix: re_compile(unix_prompt_pattern)
		prompt_cmd:  re_compile(cmd_prompt_pattern())
		prompt_pwsh: re_compile(pwsh_prompt_pattern())
		prompt_dumb: re_compile(dumb_prompt_pattern)
		prompt_any:  re_compile(win_path_prompt_pattern() + '|' + unix_prompt_pattern)
	}
	r.add_keywords(debug_words, .debug)
	r.add_keywords(warn_words, .warn)
	r.add_keywords(info_words, .info)
	r.add_keywords(success_words, .success)
	r.add_keywords(error_words, .error)
	return r
}

fn (mut r Rules) add_keywords(words []string, class Class) {
	for w in words {
		mut node := 0
		for c in w {
			key := trie_key(rune(c))
			next := int(r.trie_next[node * trie_alphabet + key])
			if next != 0 {
				node = next
				continue
			}
			new_node := r.trie_class.len
			r.trie_class << 0
			for _ in 0 .. trie_alphabet {
				r.trie_next << 0
			}
			r.trie_next[node * trie_alphabet + key] = i16(new_node)
			node = new_node
		}
		if r.trie_class[node] == 0 {
			r.trie_class[node] = u8(class)
		}
	}
}

// trie_key folds ASCII case; -1 for anything that is not a keyword char.
fn trie_key(r rune) int {
	if r >= `a` && r <= `z` {
		return int(r - `a`)
	}
	if r >= `A` && r <= `Z` {
		return int(r - `A`)
	}
	return if r == ` ` { 26 } else { -1 }
}

// longest_keyword returns the end and class of the longest keyword starting at `start`,
// or (-1, 0).
fn (r &Rules) longest_keyword(chars []rune, start int) (int, u8) {
	mut node := 0
	mut best_end := -1
	mut best_class := u8(0)
	for j in start .. chars.len {
		key := trie_key(chars[j])
		if key < 0 {
			break
		}
		next := int(r.trie_next[node * trie_alphabet + key])
		if next == 0 {
			break
		}
		node = next
		if r.trie_class[node] != 0 {
			best_end = j + 1
			best_class = r.trie_class[node]
		}
	}
	return best_end, best_class
}
