# Medians of OneTerm's `mem:` lines per scenario (bench/mem/run.sh). Run with python -I.
#   python -I bench/mem/summary.py <run dir> <scenario> ...
import json
import os
import statistics
import sys

MB = 1024 * 1024
# Which sample stands for a run: the first at or after this many seconds.
AT = {'idle': 10, 'tabs5': 10, 'lines10k': 10, 'full': 10, 'fire': 25}
FIELDS = ['private_ws', 'working_set', 'commit', 'gc_heap', 'gc_free', 'gpu_bytes',
          'grid_bytes', 'hist_bytes']


def samples(path):
    out, modules = [], []
    with open(path, encoding='utf-8', errors='replace') as f:
        for line in f:
            if line.startswith('mem: '):
                out.append(json.loads(line[5:]))
            elif line.startswith('mem-module: '):
                size, name = line[12:].strip().split(' ', 1)
                modules.append((int(size), name))
    return out, modules


def row(s):
    r = {k: s.get(k, 0) for k in FIELDS}
    kids = s.get('children') or []
    r['child_pws'] = sum(c['private_ws'] for c in kids)
    r['child_names'] = ' '.join(sorted(c['name'] for c in kids))
    r['history'] = s.get('history', 0)
    return r


def main():
    root, scenarios = sys.argv[1], sys.argv[2:]
    print('| Scenario | Private WS | WS | Commit | GC heap (free) | GPU | Grid | History (lines) | Children private WS |')
    print('|---|---|---|---|---|---|---|---|---|')
    for sc in scenarios:
        picked, peaks, mods = [], [], []
        n = 1
        while os.path.isdir(os.path.join(root, '%s-%d' % (sc, n))):
            got, modules = samples(os.path.join(root, '%s-%d' % (sc, n), 'mem.txt'))
            n += 1
            mods = modules or mods
            late = [s for s in got if s['t'] >= AT[sc]]
            if not late:
                continue
            picked.append(row(late[0]))
            peaks.append(max(s['private_ws'] for s in got))
        if not picked:
            print('| %s | no samples |' % sc)
            continue
        med = {k: statistics.median(p[k] for p in picked) for k in FIELDS + ['child_pws', 'history']}
        m = lambda k: '%.1f' % (med[k] / MB)
        print('| %s | %s | %s | %s | %s (%s) | %s | %s | %s (%d) | %s (%s) |' % (
            sc, m('private_ws'), m('working_set'), m('commit'), m('gc_heap'), m('gc_free'),
            m('gpu_bytes'), m('grid_bytes'), m('hist_bytes'), med['history'], m('child_pws'),
            picked[0]['child_names']))
        if sc == 'fire':
            print('| fire, peak private WS | %.1f | | | | | | | |' % (statistics.median(peaks) / MB))
    if mods:
        total = sum(s for s, _ in mods)
        print('\n%d modules, %.1f MB of images' % (len(mods), total / MB))


main()
