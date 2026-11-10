// Release versions and asset names. vlib semver orders 1.2.3 below 1.2.3-rc.1, so this is ours.
module update

// Version is SemVer 2.0: core, pre-release identifiers and build metadata. Comparison ignores
// build metadata: it never makes a release newer.
pub struct Version {
pub:
	major u64
	minor u64
	patch u64
	pre   []string
	build []string
}

fn numeric_identifier(text string) ?u64 {
	if text == '' || !text.contains_only('0123456789') || (text.len > 1 && text[0] == `0`) {
		return none
	}
	return text.u64()
}

fn is_identifier(text string) bool {
	return text != ''
		&& text.contains_only('0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-')
}

fn bad_version(tag string, why string) IError {
	return error("invalid release version '${tag}': ${why}")
}

// parse_release_version accepts `vMAJOR.MINOR.PATCH[-pre][+build]`.
pub fn parse_release_version(tag string) !Version {
	text := tag.trim_space().trim_left('v')
	mut rest := text
	mut build := []string{}
	if rest.contains('+') {
		build = rest.all_after('+').split('.')
		rest = rest.all_before('+')
		for id in build {
			if !is_identifier(id) {
				return bad_version(tag, 'bad build metadata')
			}
		}
	}
	mut pre := []string{}
	if rest.contains('-') {
		pre = rest.all_after('-').split('.')
		rest = rest.all_before('-')
		for id in pre {
			if !is_identifier(id) {
				return bad_version(tag, 'bad pre-release identifier')
			}
			if id.contains_only('0123456789') && numeric_identifier(id) == none {
				return bad_version(tag, 'numeric identifier with a leading zero')
			}
		}
	}
	core := rest.split('.')
	if core.len != 3 {
		return bad_version(tag, 'expected MAJOR.MINOR.PATCH')
	}
	return Version{
		major: numeric_identifier(core[0]) or { return bad_version(tag, 'bad major') }
		minor: numeric_identifier(core[1]) or { return bad_version(tag, 'bad minor') }
		patch: numeric_identifier(core[2]) or { return bad_version(tag, 'bad patch') }
		pre:   pre
		build: build
	}
}

pub fn (v Version) str() string {
	mut out := '${v.major}.${v.minor}.${v.patch}'
	if v.pre.len > 0 {
		out += '-${v.pre.join('.')}'
	}
	if v.build.len > 0 {
		out += '+${v.build.join('.')}'
	}
	return out
}

// compare is <0, 0 or >0, by SemVer precedence.
pub fn (v Version) compare(o Version) int {
	if v.major != o.major {
		return if v.major < o.major { -1 } else { 1 }
	}
	if v.minor != o.minor {
		return if v.minor < o.minor { -1 } else { 1 }
	}
	if v.patch != o.patch {
		return if v.patch < o.patch { -1 } else { 1 }
	}
	if v.pre.len == 0 || o.pre.len == 0 {
		return if v.pre.len == o.pre.len {
			0
		} else if v.pre.len == 0 {
			1
		} else {
			-1
		}
	}
	for i in 0 .. v.pre.len {
		if i >= o.pre.len {
			return 1
		}
		a := v.pre[i]
		b := o.pre[i]
		if a == b {
			continue
		}
		an := numeric_identifier(a)
		bn := numeric_identifier(b)
		if an != none && bn != none {
			return if an < bn { -1 } else { 1 }
		}
		if an != none || bn != none {
			return if an != none { -1 } else { 1 } // numeric identifiers rank lower
		}
		return if a < b { -1 } else { 1 }
	}
	return if v.pre.len == o.pre.len { 0 } else { -1 }
}

// current_target_triple is the triple this binary's release assets are named after.
pub fn current_target_triple() string {
	$if windows {
		$if arm64 {
			return 'aarch64-pc-windows-msvc'
		}
		return 'x86_64-pc-windows-msvc'
	} $else $if macos {
		$if arm64 {
			return 'aarch64-apple-darwin'
		}
		return 'x86_64-apple-darwin'
	} $else {
		$if arm64 {
			return 'aarch64-unknown-linux-gnu'
		}
		return 'x86_64-unknown-linux-gnu'
	}
}

pub fn expected_asset_name(version string, target string) string {
	normalized := version.trim_space().trim_left('v')
	if target.contains('windows') {
		return 'oneterm-${normalized}-${target}.zip'
	}
	return 'oneterm-${normalized}-${target}.tar.gz'
}
