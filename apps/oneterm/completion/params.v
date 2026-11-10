// Params is the engine's view of the settings; Config is the `completion` group of
// terminal.json: field names are the JSON keys, defaults are the original's (specs/24).
module completion

pub struct SourceToggles {
pub mut:
	memory   bool = true
	manual   bool = true
	external bool = true
}

pub struct Params {
pub mut:
	min_prefix_len           int = 1 // the option context ignores this
	max_visible_items        int = 8
	sources                  SourceToggles
	fuzzy                    bool = true
	inherit_ancestor_options bool = true
	windows_allow_coreutils  bool
	allow_fuzzy_accept       bool // off: only a prefix extension is accepted
	suggest_on_empty         bool
	redact_sensitive         bool = true
}

// hard_cap bounds the returned list to a multiple of the visible window.
pub fn (p Params) hard_cap() int {
	v := if p.max_visible_items < 1 { 1 } else { p.max_visible_items }
	return v * 4
}

pub struct Config {
pub mut:
	enabled                  bool = true
	accept_tab               bool = true
	max_history              int  = default_history_capacity
	min_prefix_len           int  = 1
	max_visible_items        int  = 8
	sources                  SourceToggles
	fuzzy                    bool = true
	inherit_ancestor_options bool = true
	disable_in_alt_screen    bool = true
	require_prompt_region    bool = true
	windows_allow_coreutils  bool
	force_family             string // '' is the original's null
	redact_sensitive         bool = true
}

// params_from_config projects the settings into the engine params.
pub fn params_from_config(c Config) Params {
	return Params{
		min_prefix_len:           c.min_prefix_len
		max_visible_items:        if c.max_visible_items < 1 { 1 } else { c.max_visible_items }
		sources:                  SourceToggles{
			memory:   c.sources.memory && c.max_history > 0
			manual:   c.sources.manual
			external: c.sources.external
		}
		fuzzy:                    c.fuzzy
		inherit_ancestor_options: c.inherit_ancestor_options
		windows_allow_coreutils:  c.windows_allow_coreutils
		redact_sensitive:         c.redact_sensitive
	}
}
