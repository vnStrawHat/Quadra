"""Compares the pictures of two folders, file by file: python tools/compare_shots.py OURS REFERENCE

For each picture in both it says what share of the pixels of the page differ by more than a
tolerance in a channel (--tolerance, 24), the worst first. A row may sit up to --drift rows
(2) higher or lower than in the reference: lines of text are whole pixels tall here, and were
not in the V pictures. By default only the page is compared: not the title bar, the sidebar
or the status bar of the gallery, which list other pages (--all compares everything). Only
the standard library.
"""
import argparse
import os
import struct
import zlib


def read(path):
    """A PNG of 8 bits a channel as (width, height, rows of RGB bytes)."""
    data = open(path, "rb").read()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", path
    at, packed, head = 8, b"", None
    while at < len(data):
        size, kind = struct.unpack(">I4s", data[at:at + 8])
        body = data[at + 8:at + 8 + size]
        if kind == b"IHDR":
            head = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            packed += body
        at += 12 + size
    width, height, depth, colour = head[:4]
    assert depth == 8 and colour in (2, 6), (path, depth, colour)
    step = 3 if colour == 2 else 4
    raw, stride = zlib.decompress(packed), width * step
    rows, before = [], bytearray(stride)
    for y in range(height):
        kind = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        if kind == 1:
            for i in range(step, stride):
                line[i] = (line[i] + line[i - step]) & 255
        elif kind == 2:
            line = bytearray((a + b) & 255 for a, b in zip(line, before))
        elif kind == 3:
            for i in range(stride):
                left = line[i - step] if i >= step else 0
                line[i] = (line[i] + (left + before[i]) // 2) & 255
        elif kind == 4:
            for i in range(stride):
                a = line[i - step] if i >= step else 0
                b, c = before[i], before[i - step] if i >= step else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        before = line
        rows.append(bytes(line) if step == 3 else bytes(v for i, v in enumerate(line) if i % 4 != 3))
    return width, height, rows


def far(mine, theirs, tolerance):
    """How many pixels of two rows of RGB bytes differ by more than the tolerance."""
    if mine == theirs:
        return 0
    off = [abs(p - q) > tolerance for p, q in zip(mine, theirs)]
    return sum(1 for i in range(0, len(off), 3) if off[i] or off[i + 1] or off[i + 2])


def differ(ours, reference, tolerance, area, drift):
    """The share of the pixels of `area` that differ, or None when the sizes do. A row is
    compared with the rows of the reference up to `drift` above and below it, and the nearest
    counts: a band that sits a pixel lower is the same band."""
    (w, h, a), (w2, h2, b) = read(ours), read(reference)
    if (w, h) != (w2, h2):
        return None
    left, top, right, bottom = area if area else (0, 0, w, h)
    bottom, count = min(bottom, h), 0
    for y in range(top, bottom):
        mine = a[y][left * 3:right * 3]
        best = far(mine, b[y][left * 3:right * 3], tolerance)
        for dy in range(1, drift + 1):
            for other in (y - dy, y + dy):
                if best and top <= other < bottom:
                    best = min(best, far(mine, b[other][left * 3:right * 3], tolerance))
        count += best
    return count / max(1, (right - left) * (bottom - top))


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("ours")
    parser.add_argument("reference")
    parser.add_argument("--tolerance", type=int, default=24)
    parser.add_argument("--drift", type=int, default=2, help="rows up and down a band may sit off (2)")
    parser.add_argument("--all", action="store_true", help="compare the whole picture, not the page alone")
    args = parser.parse_args()
    area = None if args.all else (241, 32, 1200, 770)
    names = sorted(n for n in os.listdir(args.ours) if n.endswith(".png"))
    found = []
    for name in names:
        reference = os.path.join(args.reference, name)
        if os.path.exists(reference):
            share = differ(os.path.join(args.ours, name), reference, args.tolerance, area, args.drift)
            found.append((share, name))
    for share, name in sorted(found, key=lambda f: -1 if f[0] is None else -f[0]):
        print(f"{'size' if share is None else format(share * 100, '6.2f') + ' %':>9}  {name}")
    print(f"{len(found)} compared, {len(names) - len(found)} without a reference")


main()
