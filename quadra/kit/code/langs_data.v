// Language rows: data, style and markup formats, diffs, and the table of all rows.
module code

const lang_json = LangSpec{
	names:         words('json jsonc json5 geojson webmanifest jsonl ndjson')
	constants:     words('true false null')
	line_comments: ['//']
	block_open:    '/*'
	block_close:   '*/'
	quotes:        '"'
	key_colon:     true
}

const lang_toml = LangSpec{
	names:         words('toml')
	constants:     words('true false inf nan')
	line_comments: ['#']
	quotes:        '"\''
	raw_quotes:    "'"
	triple:        true
	key_equals:    true
	sections:      true
}

const lang_ini = LangSpec{
	names:         words('ini conf cfg properties editorconfig gitconfig desktop service env dotenv')
	constants:     words('true false yes no on off')
	fold:          true
	line_comments: [';', '#']
	quotes:        '"\''
	no_dot:        true
	ident_dash:    true
	key_equals:    true
	sections:      true
}

const lang_css = LangSpec{
	names:           words('css scss sass less pcss postcss')
	keywords:        words('important')
	line_comments:   ['//']
	comment_at_word: true
	block_open:      '/*'
	block_close:     '*/'
	quotes:          '"\''
	dollar_var:      true
	at_kw:           true
	ident_dash:      true
	key_colon:       true
	selectors:       true
}

const lang_markup = LangSpec{
	names: words('html htm xhtml xml svg xsl xslt xsd plist rss atom csproj vcxproj props')
	mode:  .markup
}

const lang_diff = LangSpec{
	names: words('diff patch')
	mode:  .diff
}

const all_langs = [lang_c, lang_cpp, lang_cs, lang_java, lang_kotlin, lang_swift, lang_go, lang_rust,
	lang_js, lang_ts, lang_dart, lang_python, lang_ruby, lang_php, lang_lua, lang_sh, lang_ps,
	lang_bat, lang_sql, lang_make, lang_docker, lang_json, lang_toml, lang_ini, lang_css, lang_markup,
	lang_diff]
