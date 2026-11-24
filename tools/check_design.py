"""The design language's ladders, checked (specs/design/checklist.md): a distance, a radius or
a text size written as a number in the kit, the gallery and the apps is a step of its ladder,
or carries its reason on its line or the line before (`// design: why`).

    python tools/check_design.py          the gate's check: exits 1 when a number is off
    python tools/check_design.py --list   every number off its ladder, and those with a reason
"""
import io
import os
import re
import sys

ROOTS = ["crates/quadra-design/src", "examples/gallery/src", "apps"]
DISTANCES = {0.0, 1.0, 2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0}
RADII = {0.0, 1.0, 2.0, 4.0, 6.0, 10.0}
TEXT = {10.0, 12.0, 14.0, 16.0, 18.0, 23.0, 28.0}
NUMBER = r"(-?\d+(?:\.\d+)?)"
CHECKS = [
    (re.compile(r"\.(?:gap|gap_x|gap_y|padding|padding_x|padding_y|padding_left|padding_right"
                r"|padding_top|padding_bottom)\(" + NUMBER + r"\)"), DISTANCES, "distance"),
    (re.compile(r"\.radius\(" + NUMBER + r"\)"), RADII, "radius"),
    (re.compile(r"\.text_size\(" + NUMBER + r"\)"), TEXT, "text size"),
    (re.compile(r"\btext\([^()]*(?:\([^()]*\)[^()]*)*\)\.size\(" + NUMBER + r"\)"), TEXT, "text size"),
]


def sources():
    for root in ROOTS:
        for folder, _, names in os.walk(root):
            parts = folder.replace("\\", "/").split("/")
            if "tests" in parts or "target" in parts:
                continue
            for name in sorted(names):
                if name.endswith(".rs"):
                    yield os.path.join(folder, name).replace("\\", "/")


def main():
    off, excused = [], []
    for path in sources():
        lines = io.open(path, encoding="utf-8").read().split("\n")
        for at, line in enumerate(lines, 1):
            code = line.split("//", 1)[0]
            # The reason: on the line, or alone on the line before it.
            reason = "// design:" in line or lines[at - 2].strip().startswith("// design:")
            for pattern, ladder, kind in CHECKS:
                for found in pattern.finditer(code):
                    if float(found.group(1)) in ladder:
                        continue
                    entry = f"{path}:{at}: {kind} {found.group(1)}"
                    (excused if reason else off).append(entry)
    if "--list" in sys.argv:
        print("\n".join(off + [it + " (with a reason)" for it in excused]))
    print(f"design: {len(off)} off the ladders, {len(excused)} with a reason")
    if off and "--list" not in sys.argv:
        print("\n".join(off[:40]))
        sys.exit(1)


main()
