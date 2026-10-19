#!/usr/bin/env python3
"""Draw Method Launcher's icon from the suite's mark, and render every picture the window
and the packages need. Runs on a Mac (pictures are rendered by AppKit through
packaging/icons/render-svg.swift; iconutil builds the .icns). Standard library only.

    python3 scripts/make-icons.py

Reads assets/brand/, the suite's brand files copied unchanged from the website repo
(brand/suite/, drawn by its tools/make-brand.py). Writes:

    packaging/icons/icon.svg                 the app icon's drawing
    packaging/icons/icon-1024.png, -512, -256  the app icon (the window uses 256; Linux 256, 512)
    packaging/icons/icon.icns                macOS, every size from 16 to 1024
    packaging/icons/icon.ico                 Windows, 16 to 256
    assets/brand/wordmark.png, wordmark-dark.png
                                             the suite wordmark for the window's header

The launcher carries the suite's branding (the owner, 2026-10-19): its icon is the suite's
mark, the four apps' pages in their accents, on the family's white rounded square (a 64-unit
grid with corners of 14.3 units, inside the macOS icon grid's 100-of-1024 margin from 48
pixels up, full bleed below).
"""

from __future__ import annotations

import re
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BRAND = ROOT / "assets" / "brand"
OUT = ROOT / "packaging" / "icons"
SQUARE = "#FFFFFF"
RULE = "#DEDBD3"
SQUARE_RADIUS = 14.3 / 64
INSET = 100 / 1024
# How much of the square the mark's 64-unit box takes.
MARK = 0.9
# The wordmark's height in the header, in points, drawn at three times for sharp edges.
WORDMARK_POINTS = 34
WORDMARK_SCALE = 3


def rrect(x: float, y: float, w: float, h: float, r: float) -> str:
    return (f"M{x + r:g} {y:g}H{x + w - r:g}a{r:g} {r:g} 0 0 1 {r:g} {r:g}V{y + h - r:g}"
            f"a{r:g} {r:g} 0 0 1-{r:g} {r:g}H{x + r:g}a{r:g} {r:g} 0 0 1-{r:g}-{r:g}V{y + r:g}"
            f"a{r:g} {r:g} 0 0 1 {r:g}-{r:g}Z")


def mark_paths() -> str:
    """The suite mark's drawing, as the website draws it."""
    text = (BRAND / "mark.svg").read_text()
    paths = re.findall(r"<path [^>]*/>", text)
    if len(paths) != 4:
        raise SystemExit("assets/brand/mark.svg: expected the four pages")
    return "".join(paths)


def icon_svg(size: int) -> str:
    inset = 0.0 if size < 48 else 1024 * INSET
    side = 1024 - 2 * inset
    k = side * MARK / 64
    offset = inset + (side - 64 * k) / 2
    stroke = max(2.0, 1024 / size)
    return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">'
            f'<path fill="{SQUARE}" stroke="{RULE}" stroke-width="{stroke:g}" '
            f'd="{rrect(inset + stroke / 2, inset + stroke / 2, side - stroke, side - stroke, side * SQUARE_RADIUS)}"/>'
            f'<g transform="translate({offset:.3f} {offset:.3f}) scale({k:.5f})">{mark_paths()}</g></svg>\n')


def solid_dots(svg: str) -> str:
    """The wordmark with each i's dot as a filled disc. The website draws a dot as a stroked
    circle of radius 1, which AppKit draws as a hairline ring; only the rendering copy
    changes, never the file."""
    tag = re.search(r'<path transform="([^"]+)" fill="none" stroke="([^"]+)" stroke-width="([\d.]+)" d="([^"]+)"/>', svg)
    if not tag:
        raise SystemExit("wordmark: the lettering path has changed; update solid_dots")
    transform, ink, width, d = tag.groups()
    dot = re.compile(r"M ([\d.]+) ([\d.]+) A 1 1 0 1 1 ([\d.]+) \2 A 1 1 0 1 1 \1 \2Z ?")
    r = float(width) / 2 + 1
    discs = []
    for a, y, b in dot.findall(d):
        cx = (float(a) + float(b)) / 2
        discs.append(f'<path fill="{ink}" d="M {cx - r:g} {y} A {r:g} {r:g} 0 1 1 {cx + r:g} {y} A {r:g} {r:g} 0 1 1 {cx - r:g} {y} Z"/>')
    lettering = f'<path transform="{transform}" fill="none" stroke="{ink}" stroke-width="{width}" d="{dot.sub("", d).strip()}"/>'
    return svg.replace(tag.group(0), lettering + f'<g transform="{transform}">{"".join(discs)}</g>')


def render(jobs: list[tuple[Path, Path, int, int]]) -> None:
    args = []
    for src, png, w, h in jobs:
        args += [str(src), str(png), str(w), str(h)]
    subprocess.run(["swift", str(OUT / "render-svg.swift"), *args], check=True)


def ico(pngs: list[tuple[int, bytes]]) -> bytes:
    """A Windows .ico holding PNG pictures (Vista and later read them)."""
    head = struct.pack("<HHH", 0, 1, len(pngs))
    entries, data, offset = b"", b"", 6 + 16 * len(pngs)
    for size, png in pngs:
        dim = 0 if size >= 256 else size
        entries += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(png), offset)
        data += png
        offset += len(png)
    return head + entries + data


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "icon.svg").write_text(icon_svg(1024))
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        jobs = []

        def svg_for(px: int) -> Path:
            path = tmp / f"icon-{px}.svg"
            path.write_text(icon_svg(px))
            return path

        for px in (1024, 512, 256):
            jobs.append((svg_for(px), OUT / f"icon-{px}.png", px, px))
        iconset = tmp / "icon.iconset"
        iconset.mkdir()
        for base in (16, 32, 128, 256, 512):
            jobs.append((svg_for(base), iconset / f"icon_{base}x{base}.png", base, base))
            jobs.append((svg_for(base * 2), iconset / f"icon_{base}x{base}@2x.png", base * 2, base * 2))
        for px in (16, 32, 48):
            jobs.append((svg_for(px), tmp / f"ico-{px}.png", px, px))
        for name in ("wordmark", "wordmark-dark"):
            src = BRAND / f"{name}.svg"
            box = re.search(r'viewBox="0 0 ([\d.]+) ([\d.]+)"', src.read_text())
            if not box:
                raise SystemExit(f"{src}: no viewBox")
            h = WORDMARK_POINTS * WORDMARK_SCALE
            w = round(h * float(box.group(1)) / float(box.group(2)))
            solid = tmp / f"{name}.svg"
            solid.write_text(solid_dots(src.read_text()))
            jobs.append((solid, BRAND / f"{name}.png", w, h))
        render(jobs)
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(OUT / "icon.icns")], check=True)
        pictures = [(px, (tmp / f"ico-{px}.png").read_bytes()) for px in (16, 32, 48)]
        pictures.append((256, (OUT / "icon-256.png").read_bytes()))
        (OUT / "icon.ico").write_bytes(ico(pictures))
    for path in sorted(OUT.glob("icon*")) + sorted(BRAND.glob("*.png")):
        print(path.relative_to(ROOT))
    return 0


if __name__ == "__main__":
    sys.exit(main())
