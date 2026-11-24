"""tools/notices.py <package> <out>: the notices of what a package's binary holds, for a release
(specs/19, "Updates"): what ships beside it (tools/notices), the kit's icons and faces, and
every crate from outside the workspace the binary is built from (cargo's own graph for the
Windows target, build scripts' crates left out), each with its licence and the licence and
notice files of its source, a text that several share printed once."""
import glob
import json
import os
import subprocess
import sys


def read(path):
    return open(path, encoding="utf-8", errors="replace").read().strip()


def main():
    package, out = sys.argv[1], sys.argv[2]
    asked = ["bash", "tools/cargo.sh", "metadata", "--format-version", "1"]
    asked += ["--filter-platform", "x86_64-pc-windows-msvc"]
    meta = json.loads(subprocess.run(asked, capture_output=True, check=True, encoding="utf-8").stdout)
    crates = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    seen, todo = set(), [next(p["id"] for p in meta["packages"] if p["name"] == package)]
    while todo:
        at = todo.pop()
        if at not in seen:
            seen.add(at)
            deps = nodes[at]["deps"]
            todo += [d["pkg"] for d in deps if any(k["kind"] is None for k in d["dep_kinds"])]
    outside = sorted((crates[i] for i in seen if crates[i]["source"]), key=lambda p: (p["name"], p["version"]))

    parts = ["Notices of what this program holds and ships with."]
    parts += [read(f) for f in sorted(glob.glob("tools/notices/*.txt"))]
    kit = "crates/quadra-design"
    for name, file in [("Lucide icons", "LICENSE-lucide.txt"), ("Lilex", "fonts/OFL-lilex.txt"),
                       ("IBM Plex Sans", "fonts/OFL-plex-sans.txt")]:
        parts.append(f"{name}\n\n{read(os.path.join(kit, file))}")
    lines, texts = [], {}
    for p in outside:
        lines.append(f'{p["name"]} {p["version"]}: {p["license"] or "see its files"}, {p["repository"] or ""}')
        folder = os.path.dirname(p["manifest_path"])
        for pattern in ("LICEN[CS]E*", "COPYING*", "NOTICE*", "COPYRIGHT*"):
            for file in sorted(glob.glob(os.path.join(folder, pattern))):
                if os.path.isfile(file):
                    texts.setdefault(read(file), []).append(p["name"])
    parts.append("Rust crates built into the program:\n\n" + "\n".join(lines))
    parts += [f'{", ".join(sorted(set(names)))}\n\n{text}' for text, names in texts.items()]
    rule = "\n\n" + "-" * 78 + "\n\n"
    open(out, "w", encoding="utf-8", newline="\n").write(rule.join(parts) + "\n")
    print(f"notices: {len(outside)} crates, {len(texts)} texts in {out}")


if __name__ == "__main__":
    main()
