# Makes logo.svg beside this file (run from the project root): the mark of Markdown drawn as if still under
# construction, in what quadra-vector renders (no dashes, opacity or masks: dashes and
# hatching are drawn as short lines).
import io

GUIDE, HATCH = "#9DB0C9", "#B9C6D8"
BLUE, BLUE_EDGE, YELLOW, YELLOW_EDGE, INK = "#1E94FF", "#0076E0", "#FFD000", "#FFB800", "#0B1F33"
out = []

def dashed(x1, y1, x2, y2, dash=2.4, gap=2.0, width=0.9):
    length = ((x2 - x1) ** 2 + (y2 - y1) ** 2) ** 0.5
    ux, uy = (x2 - x1) / length, (y2 - y1) / length
    t = 0.0
    while t < length:
        e = min(t + dash, length)
        out.append(f'<line x1="{x1 + ux * t:.2f}" y1="{y1 + uy * t:.2f}" x2="{x1 + ux * e:.2f}" '
                   f'y2="{y1 + uy * e:.2f}" stroke="{GUIDE}" stroke-width="{width}"/>')
        t += dash + gap

def hatch(x0, y0, x1, y1, step=2.6, width=0.7):
    # Lines of slope -1 (rising to the right), cut to the box.
    k = x0 + y0
    while k < x1 + y1:
        pts = []
        for x, y in [(x0, k - x0), (x1, k - x1), (k - y0, y0), (k - y1, y1)]:
            if x0 - 1e-6 <= x <= x1 + 1e-6 and y0 - 1e-6 <= y <= y1 + 1e-6:
                pts.append((x, y))
        pts = sorted(set((round(x, 3), round(y, 3)) for x, y in pts))
        if len(pts) >= 2:
            (ax, ay), (bx, by) = pts[0], pts[-1]
            out.append(f'<line x1="{ax:.2f}" y1="{ay:.2f}" x2="{bx:.2f}" y2="{by:.2f}" '
                       f'stroke="{HATCH}" stroke-width="{width}"/>')
        k += step

# The construction underneath: hatched areas where the shapes will cast, and the guides.
hatch(4, 38, 22, 60)
hatch(44, 4, 60, 18)
for y in (11, 49):
    dashed(2, y, 62, y)
for x in (11, 49):
    dashed(x, 2, x, 62)

# The shape not yet placed, under the corner of the mark.
out.append(f'<circle cx="49" cy="49" r="10" fill="{YELLOW}" stroke="{YELLOW_EDGE}" stroke-width="1.4"/>')
# The mark: a blue square with M and an arrow down.
out.append(f'<rect x="11" y="11" width="38" height="38" rx="7" fill="{BLUE}" stroke="{BLUE_EDGE}" stroke-width="1.4"/>')
out.append('<polyline points="17.5,38 17.5,22 24.5,30.5 31.5,22 31.5,38" fill="none" stroke="#FFFFFF" stroke-width="3.6"/>')
out.append('<line x1="39" y1="22" x2="39" y2="36" stroke="#FFFFFF" stroke-width="3.6"/>')
out.append('<polygon points="33.5,31 44.5,31 39,38.5" fill="#FFFFFF"/>')

# The pen caught in a stroke: a curve with its nodes and a handle still showing.
out.append('<path d="M 11 11 C 20 3 34 2 49 11" fill="none" stroke="#5E9BE8" stroke-width="1.2"/>')
out.append(f'<line x1="49" y1="11" x2="57" y2="16" stroke="{BLUE_EDGE}" stroke-width="0.9"/>')
out.append(f'<circle cx="57" cy="16" r="1.8" fill="{BLUE_EDGE}"/>')
for (x, y) in ((11, 11), (49, 11)):
    out.append(f'<rect x="{x - 2.2}" y="{y - 2.2}" width="4.4" height="4.4" fill="#FFFFFF" stroke="{BLUE_EDGE}" stroke-width="1"/>')
out.append(f'<rect x="46.8" y="56.8" width="4.4" height="4.4" fill="{INK}" stroke="#FFFFFF" stroke-width="0.9"/>')

svg = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64">\n'
svg += "\n".join("  " + it for it in out) + "\n</svg>\n"
io.open("apps/markdown/assets/logo.svg", "w", encoding="utf-8", newline="\n").write(svg)
print(len(out), "shapes")

# The mark alone, for an icon under 40 px: the square with M and the arrow, and the circle,
# larger in the box; the guides, the hatching and the nodes are too fine to read there.
small = [
    f'<circle cx="49" cy="49" r="13" fill="{YELLOW}" stroke="{YELLOW_EDGE}" stroke-width="2"/>',
    f'<rect x="3" y="3" width="50" height="50" rx="9" fill="{BLUE}" stroke="{BLUE_EDGE}" stroke-width="2"/>',
    '<polyline points="11,41 11,15 20.5,27 30,15 30,41" fill="none" stroke="#FFFFFF" stroke-width="6"/>',
    '<line x1="42" y1="15" x2="42" y2="36" stroke="#FFFFFF" stroke-width="6"/>',
    '<polygon points="34,30 50,30 42,42" fill="#FFFFFF"/>',
]
svg = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64">\n'
svg += "\n".join("  " + it for it in small) + "\n</svg>\n"
io.open("apps/markdown/assets/logo-small.svg", "w", encoding="utf-8", newline="\n").write(svg)
