#!/usr/bin/env python3
"""Draw Method Launcher's icon and render the sizes the window and the packages need. Runs
on a Mac (pictures are rendered by AppKit through packaging/icons/render-svg.swift, the same
renderer Method Storyteller uses; iconutil builds the .icns).

    python3 scripts/make-icons.py

Writes to packaging/icons/:

    icon.svg                    the drawing
    icon-1024.png, -512, -256   the app icon (the window uses 256; Linux packages 256 and 512)
    icon.icns                   macOS, every size from 16 to 1024

The icon is the suite's mark, four pages for the four apps, in white on the launcher's ochre
rounded square: the door to all four. The square and its corners follow the family's app
icons (a 64-unit grid, corners of 14.3 units, inside the macOS icon grid's 100-of-1024
margin from 48 pixels up and full bleed below).
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "packaging" / "icons"
ACCENT = "#9A5B00"
SQUARE_RADIUS = 14.3 / 64
INSET = 100 / 1024
# The suite mark's four pages on its 64-unit grid: (x, y), each 18 by 24 with corners of 3.
PAGES = [(12, 6), (34, 6), (12, 34), (34, 34)]
# How much of the square's height the pages take, as the apps' marks do in their icons.
MARK = 0.6


def rrect(x: float, y: float, w: float, h: float, r: float) -> str:
    return (f"M{x + r:g} {y:g}H{x + w - r:g}a{r:g} {r:g} 0 0 1 {r:g} {r:g}V{y + h - r:g}"
            f"a{r:g} {r:g} 0 0 1-{r:g} {r:g}H{x + r:g}a{r:g} {r:g} 0 0 1-{r:g}-{r:g}V{y + r:g}"
            f"a{r:g} {r:g} 0 0 1 {r:g}-{r:g}Z")


def icon_svg(size: int) -> str:
    inset = 0.0 if size < 48 else 1024 * INSET
    side = 1024 - 2 * inset
    # The pages span 40 by 52 units; drawn MARK of the square's height, centred.
    k = side * MARK / 52
    dx = inset + (side - 40 * k) / 2 - 12 * k
    dy = inset + (side - 52 * k) / 2 - 6 * k
    pages = "".join(f'<path d="{rrect(x, y, 18, 24, 3)}"/>' for x, y in PAGES)
    return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">'
            f'<path fill="{ACCENT}" d="{rrect(inset, inset, side, side, side * SQUARE_RADIUS)}"/>'
            f'<g fill="#FFFFFF" transform="translate({dx:.3f} {dy:.3f}) scale({k:.5f})">{pages}</g></svg>\n')


def render(jobs: list[tuple[str, Path, int]]) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        args = []
        for n, (svg, png, px) in enumerate(jobs):
            src = Path(tmp) / f"{n}.svg"
            src.write_text(svg)
            args += [str(src), str(png), str(px)]
        subprocess.run(["swift", str(OUT / "render-svg.swift"), *args], check=True)


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "icon.svg").write_text(icon_svg(1024))
    render([(icon_svg(px), OUT / f"icon-{px}.png", px) for px in (1024, 512, 256)])
    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp) / "icon.iconset"
        iconset.mkdir()
        jobs = []
        for base in (16, 32, 128, 256, 512):
            jobs.append((icon_svg(base), iconset / f"icon_{base}x{base}.png", base))
            jobs.append((icon_svg(base * 2), iconset / f"icon_{base}x{base}@2x.png", base * 2))
        render(jobs)
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(OUT / "icon.icns")], check=True)
    for name in ("icon.svg", "icon-1024.png", "icon-512.png", "icon-256.png", "icon.icns"):
        print(OUT / name)
    return 0


if __name__ == "__main__":
    sys.exit(main())
