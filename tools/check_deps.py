"""Fails when a crate depends on something specs/01 (R15) does not admit.

Reads `cargo metadata --no-deps` on stdin and checks every direct dependency of every
workspace crate. Workspace crates may depend on each other.
"""
import json
import sys

# windows-core is part of windows: its `implement` macro names the crate by that name.
ADMITTED = {
    "windows", "windows-core", "accesskit", "accesskit_windows", "serde", "serde_json", "regex-lite",
    "oneterm-vt", "polling", "russh", "russh-sftp", "tokio", "zeroize", "argon2", "miniz_oxide",
}

meta = json.load(sys.stdin)
ours = {p["name"] for p in meta["packages"]}
bad = sorted(
    f'{p["name"]} -> {d["name"]}'
    for p in meta["packages"]
    for d in p["dependencies"]
    if d["name"] not in ours and d["name"] not in ADMITTED
)
if bad:
    print("check: dependencies not admitted by specs/01, R15:")
    print("\n".join("  " + b for b in bad))
    sys.exit(1)
