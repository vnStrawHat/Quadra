"""Builds Quadra's website: static pages for people, a markdown twin of each and `llms.txt`
for agents.

    tools/shots.sh dark && tools/shots.sh light    the components' pictures and words
    python tools/website.py [out]                   the site, in target/site unless told

It is made from what the repository already says: `site/` (the introduction and the guide),
`specs/design/` (the design language), and the gallery (`examples/gallery`): each of its
pages is a component's page here, with its pictures (`.tmp/shots/<theme>`), what it says of
itself and the code of its demos (`.tmp/shots/dark/pages.json`, written by the shots).
Nothing is fetched and nothing is deployed; open `index.html`.
"""
import html
import io
import json
import os
import re
import json
import shutil
import struct
import sys
import zlib

import website_md as site_md

REPO = "https://github.com/vnStrawHat/Quadra/blob/main/"
SHOTS = ".tmp/shots"
GUIDE = ["getting-started", "concepts", "views", "agents"]
DESIGN = ["README", "principles", "color", "type", "space", "layout", "controls", "surfaces", "data",
          "components", "patterns", "checklist", "decisions"]
# The roles of the kit's themes the site's stylesheet names (`--canvas`, `--text-2`, ..), and
# the kit's faces it is set in.
ROLES = ["canvas", "panel", "chrome", "panel_hover", "line", "line_strong", "text", "text_2", "text_3",
         "accent", "accent_text", "accent_tint"]
THEMES = "crates/quadra-design/themes"
FONTS = "crates/quadra-design/fonts"
FACES = [("IBM Plex Sans", "IBMPlexSans-Regular", 400, "normal"), ("IBM Plex Sans", "IBMPlexSans-Italic", 400, "italic"),
         ("IBM Plex Sans", "IBMPlexSans-SemiBold", 600, "normal"), ("Lilex", "Lilex-Regular", 400, "normal")]
# The part of a picture of the gallery that is the page: without its sidebar and its bars.
PAGE_BOX = (241, 32, 1200, 768)


def tokens():
    """The stylesheet's head: the two themes' colours as they are in the kit, and its faces."""
    def colours(name):
        palette = json.loads(read(f"{THEMES}/{name}.json"))["palette"]
        return " ".join(f"--{role.replace('_', '-')}: {palette[role]};" for role in ROLES)
    light, dark = colours("one-light"), colours("one-dark-pro")
    faces = "".join(
        f'@font-face {{ font-family: "{family}"; src: url("fonts/{file}.ttf"); font-weight: {weight}; '
        f"font-style: {style}; font-display: swap }}\n" for family, file, weight, style in FACES)
    # The viewer's own choice (`data-theme` on the root) wins over the system's, both ways.
    return (f"{faces}:root {{ {light} color-scheme: light }}\n"
            f'@media (prefers-color-scheme: dark) {{ :root:not([data-theme="light"]) {{ {dark} color-scheme: dark }} }}\n'
            f':root[data-theme="dark"] {{ {dark} color-scheme: dark }}\n')


def read(path):
    return io.open(path, encoding="utf-8").read().replace("\r\n", "\n")


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    io.open(path, "w", encoding="utf-8", newline="\n").write(text)


def picture(src, dst, box):
    """Copies a PNG, cut to `box` and packed: the shots are written unpacked."""
    data = open(src, "rb").read()
    at, chunks = 8, []
    while at < len(data):
        size, kind = struct.unpack(">I4s", data[at:at + 8])
        chunks.append((kind, data[at + 8:at + 8 + size]))
        at += 12 + size
    width, height, depth, colour = struct.unpack(">IIBB", dict(chunks)[b"IHDR"][:10])
    bpp = {2: 3, 6: 4}.get(colour)
    raw = zlib.decompress(b"".join(body for kind, body in chunks if kind == b"IDAT"))
    stride = 1 + width * (bpp or 0)
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    if depth != 8 or bpp is None or any(raw[y * stride] for y in range(height)):
        return shutil.copyfile(src, dst)  # a form this does not read: as it is
    left, top, right, bottom = (min(box[0], width), min(box[1], height), min(box[2], width), min(box[3], height))
    wide, rows = right - left, []
    line = lambda y: raw[y * stride + 1 + left * bpp:y * stride + 1 + right * bpp]
    # The empty ground under the page's last demo is left out, but for 24 px of it.
    while bottom - top > 200 and line(bottom - 1) == line(bottom - 2):
        bottom -= 1
    bottom = min(bottom + 24, min(box[3], height))
    for y in range(top, bottom):
        row = raw[y * stride + 1 + left * bpp:y * stride + 1 + right * bpp]
        rgb = bytearray(wide * 3)
        rgb[0::3], rgb[1::3], rgb[2::3] = row[0::bpp], row[1::bpp], row[2::bpp]
        rows.append(b"\0" + bytes(rgb))
    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))
    head = struct.pack(">IIBBBBB", wide, bottom - top, 8, 2, 0, 0, 0)
    packed = zlib.compress(b"".join(rows), 9)
    open(dst, "wb").write(data[:8] + chunk(b"IHDR", head) + chunk(b"IDAT", packed) + chunk(b"IEND", b""))


def include(text):
    """`{{code path}}` on a line of its own is that file, as code; `{{code path#part}}` the
    lines of it between `// #part` and `// #end`."""
    def code(found):
        path, _, part = found.group(1).partition("#")
        source = read(path).rstrip()
        if part:
            lines = source.split("\n")
            start = next(i for i, line in enumerate(lines) if line.strip() == f"// #{part}")
            end = next(i for i in range(start, len(lines)) if lines[i].strip() == "// #end")
            lines = lines[start + 1:end]
            pad = min(len(line) - len(line.lstrip()) for line in lines if line.strip())
            source = "\n".join(line[pad:] for line in lines)
        return f"```rust\n{source}\n```\n\nFrom [`{path}`]({REPO}{path}), which the gate compiles."
    return re.sub(r"^\{\{code ([^}]+)\}\}$", code, text, flags=re.M)


class Site:
    def __init__(self, out):
        self.out, self.pages, self.nav = out, [], {}

    def add(self, path, section, markdown, extra="", summary=""):
        """A page at `path` (no extension) from markdown; `extra` is HTML put after its first paragraph."""
        body, title = site_md.render(markdown)
        if extra:
            head, sep, rest = body.partition("</p>")
            body = head + sep + extra + rest
        if not summary:
            first = re.search(r"<p>(.*?)</p>", body, re.S)
            summary = re.sub(r"<[^>]+>", "", first.group(1)) if first else ""
        self.pages.append({"path": path, "section": section, "title": title, "body": body,
                           "markdown": markdown, "summary": html.unescape(summary)})
        self.nav.setdefault(section, []).append((path, title))

    def page(self, page):
        depth = page["path"].count("/")
        root = "../" * depth
        tabs = [("index", "Quadra"), ("guide/getting-started", "Guide"), ("design/README", "Design language"),
                ("components/index", "Components"), ("guide/agents", "For agents")]
        top = "".join(
            f'<a href="{root}{path}.html"{" class=on" if path == page["path"] else ""}>{name}</a>' for path, name in tabs
        )
        side = []
        for group, items in self.groups(page["section"]):
            side.append(f"<h2>{html.escape(group)}</h2>" if group else "")
            for path, title in items:
                on = ' class="on" aria-current="page"' if path == page["path"] else ""
                side.append(f'<a href="{root}{path}.html"{on}>{html.escape(title)}</a>')
        twin = os.path.basename(page["path"]) + ".md"
        return f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{html.escape(page["title"])} · Quadra</title>
<meta name="description" content="{html.escape(page["summary"][:200])}">
<link rel="alternate" type="text/markdown" href="{twin}">
<link rel="stylesheet" href="{root}site.css"></head>
<body><header><a class="mark" href="{root}index.html">Quadra</a><nav>{top}</nav></header>
<div class="frame"><aside><nav aria-label="{html.escape(page["section"])}">{"".join(side)}</nav></aside>
<main>{page["body"]}
<footer>This page as markdown: <a href="{twin}">{twin}</a> · every page for an agent: <a href="{root}llms.txt">llms.txt</a></footer>
</main></div></body></html>
"""

    def groups(self, section):
        if section == "Quadra":
            more = [("design/README", "The design language"), ("components/index", "Components")]
            return [("Start here", self.nav["Quadra"] + self.nav.get("Guide", [])), ("Reference", more)]
        if section != "Components":
            return [(section, self.nav.get(section, []))]
        return [("Components", [("components/index", "All components")])] + self.component_groups

    def build(self):
        for page in self.pages:
            write(f"{self.out}/{page['path']}.html", self.page(page))
            write(f"{self.out}/{page['path']}.md", page["markdown"])
        write(f"{self.out}/site.css", tokens() + read("site/site.css"))
        os.makedirs(f"{self.out}/fonts", exist_ok=True)
        for _, file, _, _ in FACES:
            shutil.copyfile(f"{FONTS}/{file}.ttf", f"{self.out}/fonts/{file}.ttf")
        for licence in ("OFL-plex-sans.txt", "OFL-lilex.txt"):
            shutil.copyfile(f"{FONTS}/{licence}", f"{self.out}/fonts/{licence}")
        lines = ["# Quadra", "", "> " + self.pages[0]["summary"], "",
                 "Every page of the site has a markdown twin: the same path ending in `.md`. "
                 "`llms-full.txt` is all of them in one file.", ""]
        for section in dict.fromkeys(page["section"] for page in self.pages):
            lines += [f"## {section}", ""]
            lines += [f"- [{p['title']}]({p['path']}.md): {p['summary']}" for p in self.pages if p["section"] == section]
            lines.append("")
        write(f"{self.out}/llms.txt", "\n".join(lines))
        full = [f"<!-- {p['path']}.md -->\n\n{p['markdown'].strip()}\n" for p in self.pages]
        write(f"{self.out}/llms-full.txt", "\n\n".join(full))


def shots(out, ident):
    """The pictures of a gallery page, copied into the site: the names of those there are."""
    found = []
    for name in (ident, ident + "-open"):
        if all(os.path.exists(f"{SHOTS}/{theme}/{name}.png") for theme in ("dark", "light")):
            for theme in ("dark", "light"):
                picture(f"{SHOTS}/{theme}/{name}.png", f"{out}/img/{theme}/{name}.png", PAGE_BOX)
            found.append(name)
    return found


def figure(root, name, words):
    alt = html.escape(words)
    return (f'<figure><picture><source srcset="{root}img/dark/{name}.png" media="(prefers-color-scheme: dark)">'
            f'<img src="{root}img/light/{name}.png" alt="{alt}" loading="lazy"></picture>'
            f"<figcaption>{alt}</figcaption></figure>")


def components(site):
    pages = json.loads(read(f"{SHOTS}/dark/pages.json"))
    made = {i: (m, f) for i, m, f in re.findall(r'"([a-z0-9-]+)" => (\w+)::(\w+)\(', read("examples/gallery/src/registry.rs"))}
    groups, cards = {}, []
    for page in pages:
        ident, title, group = page["id"], page["title"], page["group"]
        if group == "Performance":
            continue
        names = shots(site.out, ident)
        module, function = made.get(ident, ("overview", "start_page"))
        source = f"examples/gallery/src/pages/{module}.rs"
        whole = read(source)
        start = whole.find(f"pub fn {function}(")
        built = whole[start:whole.find("\n}\n", start) + 2] if start >= 0 else ""
        text = [f"# {title}", "", page["about"] or f"{title}, as the gallery shows it.", ""]
        text += [f"Pictures: " + ", ".join(f"[{n}, {t}](../img/{t}/{n}.png)" for n in names for t in ("dark", "light")), ""]
        if any(demo["code"] for demo in page["demos"]):
            text += ["## How it is written", ""]
        for demo in page["demos"]:
            if demo["code"]:
                text += [f"### {demo['title']}", "", "```rust", demo["code"], "```", ""]
        if built:
            text += ["## The page's code", "", f"The function that builds the page pictured above, as the gate compiles it "
                     f"(what it calls and is not the kit's is beside it in [`{source}`]({REPO}{source})):", "",
                     "```rust", built, "```", ""]
        text += ["## More", "", f"- The file this page is in, whole: [`{source}`]({REPO}{source}).",
                 "- What it looks like and why: [the design language](../design/README.md); "
                 "its API, keys and semantics: [`specs/13-kit.md`](" + REPO + "specs/13-kit.md).",
                 "- Everything here is `quadra_design::..`; `use quadra_design::*` and `use quadra::*` bring the names in.", ""]
        extra = "".join(figure("../", n, f"{title}{', open' if n.endswith('-open') else ''}") for n in names)
        site.add(f"components/{ident}", "Components", "\n".join(text), extra, page["about"])
        groups.setdefault(group, []).append((f"components/{ident}", title))
        cards.append((group, ident, title, page["about"], bool(names)))
    site.component_groups = list(groups.items())
    listing = ["# Components", "", f"The {len(cards)} pages of the gallery: every component of the kit "
               "(`quadra-design`) and of the crates beside it, with what it looks like and how it is written.", ""]
    grid = []
    for group in groups:
        listing += [f"## {group}", ""]
        grid.append(f'<h2 id="{site_md.slug(group)}">{html.escape(group)}</h2><div class="cards">')
        for g, ident, title, about, has in cards:
            if g != group:
                continue
            listing.append(f"- [{title}]({ident}.md): {about}")
            thumb = (f'<picture><source srcset="../img/dark/{ident}.png" media="(prefers-color-scheme: dark)">'
                     f'<img src="../img/light/{ident}.png" alt="" loading="lazy"></picture>') if has else ""
            grid.append(f'<a class="card" href="{ident}.html">{thumb}<strong>{html.escape(title)}</strong>'
                        f"<span>{html.escape(about.split('. ')[0])}</span></a>")
        listing.append("")
        grid.append("</div>")
    site.add("components/index", "Components", "\n".join(listing))
    index = site.pages[-1]
    index["body"] = index["body"].split("<h2", 1)[0] + "".join(grid)


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "target/site"
    shutil.rmtree(out, ignore_errors=True)
    site = Site(out)
    site.component_groups = []
    site.add("index", "Quadra", include(read("site/index.md")))
    for name in GUIDE:
        site.add(f"guide/{name}", "Guide", include(read(f"site/guide/{name}.md")))
    for name in DESIGN:
        site.add(f"design/{name}", "Design language", read(f"specs/design/{name}.md"))
    components(site)
    site.build()
    size = sum(os.path.getsize(os.path.join(d, f)) for d, _, fs in os.walk(out) for f in fs)
    print(f"site: {len(site.pages)} pages in {out}, {size // 1024} KB")


if __name__ == "__main__":
    main()
