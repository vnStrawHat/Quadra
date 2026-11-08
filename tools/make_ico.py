"""Generate a simple app icon (.ico holding a 256 px PNG): a rounded square in the accent
colour with a white note page. Usage: python tools/make_ico.py out.ico [RRGGBB]"""
import struct
import sys
import zlib

SIZE = 256


def clamp(v):
    return 0.0 if v < 0 else 1.0 if v > 1 else v


def rounded_box(x, y, cx, cy, hw, hh, r):
    """Signed distance to a rounded rectangle (negative inside)."""
    dx = abs(x - cx) - (hw - r)
    dy = abs(y - cy) - (hh - r)
    ox, oy = max(dx, 0.0), max(dy, 0.0)
    return (ox * ox + oy * oy) ** 0.5 + min(max(dx, dy), 0.0) - r


def pixel(x, y, accent):
    px, py = x + 0.5, y + 0.5
    bg = clamp(0.5 - rounded_box(px, py, 128, 128, 120, 120, 52))
    page = clamp(0.5 - rounded_box(px, py, 128, 132, 62, 78, 12))
    lines = 0.0
    for ly in (96, 122, 148, 174):
        width = 70 if ly != 174 else 44
        d = rounded_box(px, py, 128 - (70 - width) / 2 * 0.9, ly, width / 2 * 0.9 + 4, 5, 5)
        lines = max(lines, clamp(0.5 - d))
    r, g, b = accent
    # Page over background, lines in the accent colour over the page.
    cr = r + (255 - r) * page - (255 - r) * lines * page
    cg = g + (255 - g) * page - (255 - g) * lines * page
    cb = b + (255 - b) * page - (255 - b) * lines * page
    return int(cr), int(cg), int(cb), int(255 * bg)


def png(accent):
    rows = bytearray()
    for y in range(SIZE):
        rows.append(0)
        for x in range(SIZE):
            rows.extend(pixel(x, y, accent))

    def chunk(kind, data):
        c = kind + data
        return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xFFFFFFFF)

    ihdr = struct.pack('>IIBBBBB', SIZE, SIZE, 8, 6, 0, 0, 0)
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', ihdr) +
            chunk(b'IDAT', zlib.compress(bytes(rows), 9)) + chunk(b'IEND', b''))


def main():
    out = sys.argv[1]
    hexc = sys.argv[2] if len(sys.argv) > 2 else '2563EB'
    accent = tuple(int(hexc[i:i + 2], 16) for i in (0, 2, 4))
    data = png(accent)
    header = struct.pack('<HHH', 0, 1, 1)
    entry = struct.pack('<BBBBHHII', 0, 0, 0, 0, 1, 32, len(data), 6 + 16)
    with open(out, 'wb') as f:
        f.write(header + entry + data)


if __name__ == '__main__':
    main()
