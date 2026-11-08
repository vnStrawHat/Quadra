"""Exact-replacement editor: PYTHONUTF8=1 python -I tools/ed.py <spec.py>. The spec defines EDITS as a
list of (path, [(old, new), ...]) and optional APPEND as a list of (path, text)."""
import io
import sys

ns = {}
exec(io.open(sys.argv[1], encoding="utf-8").read(), ns)
for path, reps in ns.get("EDITS", []):
    s = io.open(path, encoding="utf-8", newline="").read()
    for a, b in reps:
        n = s.count(a)
        if n != 1:
            sys.exit("%s: %d matches for %r" % (path, n, a[:70]))
        s = s.replace(a, b)
    io.open(path, "w", encoding="utf-8", newline="").write(s)
for path, text in ns.get("APPEND", []):
    s = io.open(path, encoding="utf-8", newline="").read()
    io.open(path, "w", encoding="utf-8", newline="").write(s + text)
print("ok")
