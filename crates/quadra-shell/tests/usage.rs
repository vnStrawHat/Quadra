//! What the process used (specs/16): Windows reads it.

#![cfg(windows)]

use std::time::Duration;

#[test]
fn the_process_says_what_it_used() {
    let usage = quadra_shell::process_usage().expect("Windows reads it");
    assert!(usage.memory > 64 * 1024, "a private working set of more than 64 KB: {usage:?}");
    assert!(usage.memory <= usage.working_set && usage.working_set <= usage.peak_working_set, "{usage:?}");
    assert!(usage.private_bytes > 0 && usage.threads >= 1, "{usage:?}");
    assert!(usage.uptime > Duration::ZERO && usage.uptime < Duration::from_secs(24 * 3600), "{usage:?}");
}
