"""Markdown to HTML for the site (tools/website.py): the subset the specs and the site's pages
are written in. Headings, paragraphs, lists (nested), tables, fenced code, and inline code,
bold and links. A link to a `.md` file becomes one to its `.html` page.

    python tools/website_md.py    checks itself
"""
import html
import re

INLINE = re.compile(r"`([^`]+)`|\*\*([^*]+)\*\*|\[([^\]]+)\]\(([^)\s]+)\)")
ITEM = re.compile(r"^(\s*)([-*]|\d+\.)\s+(.*)$")


def slug(words):
    return re.sub(r"[^a-z0-9]+", "-", re.sub(r"[`*]", "", words).lower()).strip("-")


def inline(text):
    out, at = [], 0
    for m in INLINE.finditer(text):
        out.append(html.escape(text[at:m.start()], quote=False))
        code, bold, label, url = m.groups()
        if code is not None:
            out.append(f"<code>{html.escape(code, quote=False)}</code>")
        elif bold is not None:
            out.append(f"<strong>{inline(bold)}</strong>")
        else:
            if not re.match(r"[a-z]+:", url):
                url = re.sub(r"\.md(#|$)", r".html\1", url)
            out.append(f'<a href="{html.escape(url)}">{inline(label)}</a>')
        at = m.end()
    out.append(html.escape(text[at:], quote=False))
    return "".join(out)


def cells(line):
    return [c.strip() for c in re.split(r"(?<!\\)\|", line.strip().strip("|"))]


def table(lines):
    head, rows = cells(lines[0]), [cells(l) for l in lines[2:]]
    out = ['<div class="table"><table><thead><tr>']
    out += [f"<th>{inline(c)}</th>" for c in head] + ["</tr></thead><tbody>"]
    for row in rows:
        out.append("<tr>" + "".join(f"<td>{inline(c)}</td>" for c in row) + "</tr>")
    return "".join(out) + "</tbody></table></div>"


def blocks(lines):
    """The blocks of `lines`, as HTML."""
    out, i = [], 0
    while i < len(lines):
        line = lines[i]
        if not line.strip():
            i += 1
        elif line.lstrip().startswith("```"):
            lang, j = line.strip()[3:].strip(), i + 1
            while j < len(lines) and not lines[j].lstrip().startswith("```"):
                j += 1
            pad = len(line) - len(line.lstrip())
            code = "\n".join(l[pad:] if l[:pad].isspace() else l for l in lines[i + 1:j])
            out.append(f'<pre data-lang="{html.escape(lang)}"><code>{html.escape(code, quote=False)}</code></pre>')
            i = j + 1
        elif re.match(r"#{1,4} ", line):
            level, words = len(line.split(" ")[0]), line.split(" ", 1)[1].strip()
            out.append(f'<h{level} id="{slug(words)}">{inline(words)}</h{level}>')
            i += 1
        elif line.lstrip().startswith("|") and i + 1 < len(lines) and re.match(r"\s*\|[\s:|-]+\|\s*$", lines[i + 1]):
            j = i
            while j < len(lines) and lines[j].lstrip().startswith("|"):
                j += 1
            out.append(table(lines[i:j]))
            i = j
        elif ITEM.match(line):
            indent, ordered = len(ITEM.match(line).group(1)), ITEM.match(line).group(2)[0].isdigit()
            items = []
            while i < len(lines):
                m = ITEM.match(lines[i])
                if m and len(m.group(1)) == indent:
                    items.append([m.group(3)])
                elif lines[i].strip() and len(lines[i]) - len(lines[i].lstrip()) > indent:
                    items[-1].append(lines[i][indent + 2:] if lines[i][:indent + 2].isspace() else lines[i].lstrip())
                elif not lines[i].strip() and i + 1 < len(lines) and lines[i + 1].strip() and (
                    len(lines[i + 1]) - len(lines[i + 1].lstrip()) > indent
                ):
                    items[-1].append("")
                else:
                    break
                i += 1
            tag = "ol" if ordered else "ul"
            inner = []
            for item in items:
                body = blocks(item)
                if body.count("<p>") == 1 and body.startswith("<p>") and body.endswith("</p>"):
                    body = body[3:-4]
                inner.append(f"<li>{body}</li>")
            out.append(f"<{tag}>{''.join(inner)}</{tag}>")
        else:
            j = i
            while j < len(lines) and lines[j].strip() and not re.match(r"#{1,4} |\s*```|\s*\|", lines[j]) and not (
                j > i and ITEM.match(lines[j])
            ):
                j += 1
            j = max(j, i + 1)
            out.append("<p>" + inline(" ".join(l.strip() for l in lines[i:j])) + "</p>")
            i = j
    return "\n".join(out)


def render(text):
    """The HTML of a markdown text, and its title (its first heading)."""
    lines = text.replace("\r\n", "\n").split("\n")
    title = next((l[2:].strip() for l in lines if l.startswith("# ")), "")
    return blocks(lines), re.sub(r"[`*]", "", title)


def demo():
    body, title = render("# A `b`\n\nOne **two** [x](y.md#z).\n\n- a\n  more\n  - in\n- b\n\n| h | i |\n|---|---|\n| 1 | `2` |\n\n```rust\nlet a = 1 < 2;\n```\n")
    assert title == "A b", title
    assert '<h1 id="a-b">A <code>b</code></h1>' in body, body
    assert '<a href="y.html#z">x</a>' in body and "<strong>two</strong>" in body
    assert "<ul><li><p>a more</p>\n<ul><li>in</li></ul></li><li>b</li></ul>" in body, body
    assert "<td><code>2</code></td>" in body and "let a = 1 &lt; 2;" in body


if __name__ == "__main__":
    demo()
    print("website_md: ok")
