//! What the process used (specs/16): the system reads it.

#![cfg(any(windows, target_os = "macos"))]

use std::time::Duration;

#[test]
fn the_process_says_what_it_used() {
    let usage = quadra_shell::process_usage().expect("the system reads it");
    assert!(usage.memory > 64 * 1024, "a private working set of more than 64 KB: {usage:?}");
    // The memory macOS counts as the process's holds what it compressed, which is in no RAM.
    assert!(cfg!(target_os = "macos") || usage.memory <= usage.working_set, "{usage:?}");
    assert!(usage.working_set <= usage.peak_working_set, "{usage:?}");
    assert!(usage.private_bytes > 0 && usage.threads >= 1, "{usage:?}");
    assert!(usage.uptime > Duration::ZERO && usage.uptime < Duration::from_secs(24 * 3600), "{usage:?}");
}
