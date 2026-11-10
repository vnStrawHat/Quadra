// The crash report store: completed `*.crash.txt` files under `crashes/`, reconciled at start
// (specs/25). An elevated (read-only) process keeps its own store, `crashes/elevated/`.
module crash

import os
import quadra.native.sys
import rand
import time

const crashes_dir_name = 'crashes'
const elevated_dir_name = 'elevated'
const completed_suffix = '.crash.txt'
const native_suffix = '.native.tmp'
const stderr_suffix = '.stderr.tmp'
const max_reports = 20
const redacted_user_home = '<USER_HOME>'
const panic_marker = 'V panic' // what V's panic() prints to stderr

pub struct PendingCrashReport {
pub:
	path     string
	contents string
}

// CrashCapturePaths names this process's staging files, chosen before any callback can run.
pub struct CrashCapturePaths {
pub:
	identity string
	native   string
	stderr   string
}

// crashes_dir_in is the one definition of the store, so the writer, the reader and the path
// validator cannot drift apart.
pub fn crashes_dir_in(config_dir string, elevated bool) string {
	crashes := os.join_path(config_dir, crashes_dir_name)
	return if elevated { os.join_path(crashes, elevated_dir_name) } else { crashes }
}

// prepare_capture_paths creates the store and allocates a collision-resistant identity.
pub fn prepare_capture_paths(config_dir string, elevated bool) !CrashCapturePaths {
	directory := crashes_dir_in(config_dir, elevated)
	os.mkdir_all(directory)!
	identity := unique_identity(directory)!
	return CrashCapturePaths{
		identity: identity
		native:   os.join_path(directory, identity + native_suffix)
		stderr:   os.join_path(directory, identity + stderr_suffix)
	}
}

fn unique_identity(directory string) !string {
	t := time.utc()
	stamp := '${t.year:04}${t.month:02}${t.day:02}T${t.hour:02}${t.minute:02}${t.second:02}${t.nanosecond / 1_000_000:03}Z'
	for _ in 0 .. 32 {
		identity := '${stamp}-p${os.getpid()}-${rand.u32():08x}'
		if ![completed_suffix, native_suffix, stderr_suffix].any(os.exists(os.join_path(directory,

			identity + it))) {
			return identity
		}
	}
	return error('failed to allocate a unique crash report identity')
}

// load_pending_reports promotes staging left by dead processes, sanitizes every report and keeps
// the newest 20, newest first. `own_identity` is this process's identity, always skipped.
pub fn load_pending_reports(config_dir string, elevated bool, home ?string, own_identity string) ![]PendingCrashReport {
	directory := crashes_dir_in(config_dir, elevated)
	os.mkdir_all(directory)!
	promote_inactive_staging(directory, own_identity)!
	return load_completed_reports(directory, home)!
}

// delete_pending_report removes one report; refused outside the managed store.
pub fn delete_pending_report(config_dir string, elevated bool, path string) ! {
	if os.dir(path) != crashes_dir_in(config_dir, elevated) || !path.ends_with(completed_suffix) {
		return error('crash report path is outside the managed crash store')
	}
	remove_if_present(path)!
}

fn promote_inactive_staging(directory string, own_identity string) ! {
	current_pid := u32(os.getpid())
	mut is_panic_staging := map[string]bool{}
	mut foreign := []string{}
	for name in report_names_with_suffix(directory, native_suffix)! {
		foreign << name
	}
	for name in report_names_with_suffix(directory, stderr_suffix)! {
		foreign << name
		is_panic_staging[name] = true
	}
	for name in foreign {
		panic_staging := is_panic_staging[name]
		identity := name.all_before_last('.').all_before_last('.')
		if identity == own_identity {
			continue
		}
		owner_pid := pid_from_identity(identity) or { continue }
		if owner_pid != current_pid && sys.process_is_live(owner_pid) {
			continue
		}
		source := os.join_path(directory, name)
		claimed := os.join_path(directory,
			'${identity}.native.claimed-p${current_pid}-${rand.u32():08x}.tmp')
		os.rename(source, claimed) or {
			if !os.exists(source) {
				continue // another instance claimed it first
			}
			return err
		}
		promote_claimed(identity, claimed, directory, panic_staging)!
	}
}

fn pid_from_identity(identity string) ?u32 {
	rest := identity.all_after_last('-p')
	if !rest.contains('-') || rest == identity {
		return none
	}
	pid := rest.all_before('-')
	return if pid != '' && pid.contains_only('0123456789') { pid.u32() } else { none }
}

fn promote_claimed(identity string, claimed string, directory string, panic_staging bool) ! {
	staged := os.read_bytes(claimed)!
	// A panic staging file always holds its header; only a real panic is a crash.
	if staged.len == 0 || (panic_staging && !staged.bytestr().contains(panic_marker)) {
		remove_if_present(claimed)!
		return
	}
	completed := os.join_path(directory, identity + completed_suffix)
	mut combined := []u8{}
	if os.exists(completed) {
		existing := os.read_bytes(completed)!
		if existing.len > 0 {
			combined << existing
			combined << '\n\n--- Native crash context ---\n'.bytes()
		}
	}
	combined << staged
	if panic_staging {
		combined << 'Timestamp (Unix): ${os.file_last_mod_unix(claimed)}\n'.bytes()
	}
	os.write_file_array(completed, combined)!
	remove_if_present(claimed)!
}

fn load_completed_reports(directory string, home ?string) ![]PendingCrashReport {
	mut names := report_names_with_suffix(directory, completed_suffix)!
	names.sort(a > b)
	// One unreadable or undeletable file must not hide every other report.
	for name in names[int_min(max_reports, names.len)..] {
		remove_if_present(os.join_path(directory, name)) or {
			eprintln('failed to prune old crash report ${name}: ${err.msg()}')
		}
	}
	mut reports := []PendingCrashReport{}
	for name in names[..int_min(max_reports, names.len)] {
		path := os.join_path(directory, name)
		contents := load_and_sanitize_report(path, home) or {
			eprintln('failed to load crash report ${name}: ${err.msg()}')
			continue
		}
		if contents != '' {
			reports << PendingCrashReport{path, contents}
		}
	}
	return reports
}

fn int_min(a int, b int) int {
	return if a < b { a } else { b }
}

fn report_names_with_suffix(directory string, suffix string) ![]string {
	return os.ls(directory)!.filter(it.ends_with(suffix) && os.is_file(os.join_path(directory, it)))
}

// load_and_sanitize_report redacts the home prefix and rewrites a report that still held it. An
// empty report is deleted; '' then.
fn load_and_sanitize_report(path string, home ?string) !string {
	if !os.exists(path) {
		return ''
	}
	report := os.read_file(path)!
	if report == '' {
		remove_if_present(path)!
		return ''
	}
	sanitized := redact_user_home(report, home)
	if sanitized != report {
		os.write_file(path, sanitized)!
	}
	return sanitized
}

fn remove_if_present(path string) ! {
	if os.exists(path) {
		os.rm(path)!
	}
}

// redact_user_home replaces every occurrence of the home prefix with <USER_HOME>, in both
// separators, ASCII case-insensitively on Windows.
pub fn redact_user_home(report string, home ?string) string {
	root := home or { return report }
	if root.len <= 3 || root == '/' || root == '\\' {
		return report
	}
	mut sanitized := report
	for variant in [root, root.replace('\\', '/'), root.replace('/', '\\')] {
		sanitized = replace_home_variant(sanitized, variant)
	}
	return sanitized
}

fn replace_home_variant(value string, variant string) string {
	$if windows {
		return replace_ascii_case_insensitive(value, variant, redacted_user_home)
	} $else {
		return value.replace(variant, redacted_user_home)
	}
}

fn replace_ascii_case_insensitive(value string, pattern string, replacement string) string {
	lowered := value.to_lower_ascii()
	wanted := pattern.to_lower_ascii()
	mut out := []u8{cap: value.len}
	mut start := 0
	for {
		found := lowered.index_after(wanted, start) or { break }
		if found < 0 {
			break
		}
		out << value[start..found].bytes()
		out << replacement.bytes()
		start = found + pattern.len
	}
	out << value[start..].bytes()
	return out.bytestr()
}

const issue_title = 'Crash report'

// new_issue_url is the title-prefilled GitHub draft. The report itself never goes in the URL.
pub fn new_issue_url(repository string) string {
	return 'https://github.com/${repository}/issues/new?title=${issue_title.replace(' ', '%20')}'
}
