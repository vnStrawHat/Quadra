#!/usr/bin/env python3
"""Converts the original's bundled themes into Quadra theme files (specs/22, Config).

Run from the project root with PYTHONUTF8=1:
    python -I apps/oneterm/themes/convert.py <folder of the original's themes>

Writes apps/oneterm/themes/<slug>.theme.json for each variant and apps/oneterm/themes_embedded.v.
Words in .private/forbidden-names.txt are dropped from theme names.
"""
import colorsys
import glob
import json
import os
import re
import sys

OUT = 'apps/oneterm/themes'
HEX = re.compile(r'#([0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{3})\b')


def color(value):
    """The first hex color in a theme value (a gradient gives its start), or None."""
    m = HEX.search(str(value))
    if not m:
        return None
    h = m.group(1)
    if len(h) == 3:
        h = ''.join(c * 2 for c in h)
    return '#' + h.upper()


def mix(a, b, t):
    """a blended toward b by t, opaque."""
    x = [int(a[i:i + 2], 16) for i in (1, 3, 5)]
    y = [int(b[i:i + 2], 16) for i in (1, 3, 5)]
    return '#' + ''.join('%02X' % round(p + (q - p) * t) for p, q in zip(x, y))


def alpha(a, t):
    return a[:7] + '%02X' % round(255 * t)


def clamp_alpha(a, top):
    """The original caps the selection's opacity at top."""
    have = int(a[7:9], 16) / 255 if len(a) == 9 else 1
    return alpha(a, min(have, top))


def darken(a, t):
    """The original's Hsla darken: lightness times 1 - t."""
    r, g, b = (int(a[i:i + 2], 16) / 255 for i in (1, 3, 5))
    h, l, s = colorsys.rgb_to_hls(r, g, b)
    return '#' + ''.join('%02X' % round(v * 255) for v in colorsys.hls_to_rgb(h, l * (1 - t), s))


def convert(t):
    # Only keys of the original's theme schema count; it ignores others (panel.background,
    # link.foreground), and so does this. Fallbacks follow the original's where it has one.
    c = {k: color(v) for k, v in t['colors'].items()}
    c = {k: v for k, v in c.items() if v}

    def first(*keys):
        for k in keys:
            if k in c:
                return c[k]
        return None

    bg = first('background')
    fg = first('foreground')
    dark = t['mode'] == 'dark'
    border = first('border', 'input.border') or mix(bg, fg, 0.2)
    muted = first('muted.foreground', 'tab.foreground') or mix(fg, bg, 0.3)
    accent = first('primary.background', 'ring', 'base.blue')
    panel = first('muted.background') or bg
    chrome = first('title_bar.background') or bg
    tab_bg = first('tab_bar.background') or bg
    hover = first('list.hover.background') or mix(bg, first('accent.background') or fg, 0.6)
    raised = first('secondary.background', 'accent.background') or mix(bg, fg, 0.05)
    press = first('list.active.background', 'accent.background') or mix(bg, fg, 0.12)
    over = first('popover.background') or bg
    palette = {
        'canvas': bg,
        'panel': panel,
        'panel_hover': hover,
        'raised': raised,
        'raised_hover': first('secondary.hover.background') or hover,
        'press': press,
        'overlay_bg': over,
        'scrim': first('overlay') or ('#00000033' if dark else '#0000000D'),
        'line': border,
        'line_strong': border,
        'border_focused': first('ring', 'base.blue') or accent,
        'chrome': chrome,
        'status_bg': first('status_bar.background') or chrome,
        'tab_bg': tab_bg,
        'scroll_thumb': first('scrollbar.thumb.background') or alpha(fg, 0.3),
        'text': fg,
        'text_2': muted,
        'text_3': mix(muted, bg, 0.2),
        'text_disabled': mix(muted, bg, 0.45),
        'text_on_accent': first('primary.foreground') or bg,
        'accent': accent,
        'accent_hover': first('primary.hover.background') or mix(bg, accent, 0.9),
        'accent_press': first('primary.active.background') or darken(accent, 0.2 if dark else 0.1),
        'accent_tint': alpha(accent, 0.25),
        'accent_text': first('link') or accent,
        'ok': first('success.background', 'base.green'),
        'warn': first('warning.background', 'base.yellow'),
        'error': first('danger.background', 'base.red'),
        'info': first('info.background', 'base.cyan'),
        # Roles the kit derives when absent (theme_roles.v); given here as the original resolves them.
        'selection': clamp_alpha(first('selection.background') or accent, 0.3),
        'tab_fg': first('tab.foreground') or fg,
        'tab_active_fg': first('tab.active.foreground') or fg,
        'table_head_bg': first('table.head.background', 'list.head.background', 'list.background') or bg,
        'table_head_fg': first('table.head.foreground') or muted,
        'table_row_border': first('table.row.border') or border,
        'list_even': first('list.even.background', 'list.background') or bg,
        'scroll_thumb_hover': first('scrollbar.thumb.hover.background', 'scrollbar.thumb.background'),
        'text_on_status': first('danger.foreground', 'primary.foreground'),
        'error_hover': first('danger.hover.background') or mix(bg, first('danger.background', 'base.red'), 0.9),
        'cyan': first('base.cyan'),
    }
    series = [first('chart.%d' % (i + 1), key) for i, key in
              enumerate(['base.blue', 'base.green', 'base.yellow', 'base.magenta', 'base.red'])]
    selection = first('selection.background') or alpha(accent, 0.3)
    return {
        'name': t['name'],
        'mode': 'dark' if dark else 'light',
        'palette': {k: v for k, v in palette.items() if v},
        'editor': {'foreground': fg, 'selection': selection},
        'terminal': {'foreground': fg, 'background': bg, 'cursor': first('caret') or accent,
                     'selection': selection},
        'series': [s for s in series if s],
    }


def main():
    src = sys.argv[1]
    with open('.private/forbidden-names.txt', encoding='utf-8') as f:
        banned = [w.strip() for w in f if w.strip()]
    gone = re.compile(r'\b(' + '|'.join(map(re.escape, banned)) + r')\b', re.I)
    themes = []
    for path in sorted(glob.glob(os.path.join(src, '*.json'))):
        with open(path, encoding='utf-8') as f:
            for t in json.load(f)['themes']:
                t['name'] = ' '.join(gone.sub('', t['name']).split())
                themes.append(convert(t))
    lines = ['// Generated by themes/convert.py: do not edit.', 'module main', '',
             'const bundled_themes = [']
    for t in themes:
        slug = re.sub(r'[^a-z0-9]+', '-', t['name'].lower()).strip('-')
        # .theme.json: V merges two $embed_file entries with the same relative path, and the kit
        # also embeds themes/one-light.json.
        slug += '.theme'
        with open(os.path.join(OUT, slug + '.json'), 'w', encoding='utf-8', newline='') as f:
            json.dump(t, f, indent='\t')
            f.write('\n')
        lines.append("\tBundledTheme{'%s', '%s', $embed_file('themes/%s.json').to_string()}," %
                     (t['name'], t['mode'], slug))
    lines += [']', '']
    with open('apps/oneterm/themes_embedded.v', 'w', encoding='utf-8', newline='') as f:
        f.write('\n'.join(lines))
    print(len(themes), 'themes')


main()
