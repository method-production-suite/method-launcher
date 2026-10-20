#!/usr/bin/env python3
"""Draw Method Launcher's mark, wordmark and favicon. Standard library only.

    python3 scripts/make-brand.py           write the files below
    python3 scripts/make-brand.py --check   change nothing; exit 1 if a file is out of date

Writes:

    site/assets/brand/mark.svg, mark-dark.svg          the mark: the suite's four pages, one
                                                       per app, as on the app icon
    site/assets/brand/wordmark.svg, wordmark-dark.svg  the mark and "method launcher"
    site/favicon.svg                                   the mark on the 16-unit grid
    assets/brand/launcher-wordmark.svg, -dark.svg      the wordmark again, for the README

The launcher opens every app, so its mark is the suite's: the four apps' accents as four
pages. The letters are scripts/glyphs.py, the family's strokes, set the way the website's
tools/make-brand.py sets the suite's name.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from glyphs import DESC_END, W, flip, fmt, rrect, set_name  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
INK, INK_DARK = "#1C2228", "#E4E8EC"
NAME, LABEL = "method launcher", "Method Launcher"

# Each app's accent, light and dark (the apps' own tokens.css), in the icon's order.
ACCENTS = [("#0E7468", "#2CC7B2"), ("#2F5FB3", "#7AA2F0"), ("#7052BA", "#B49CF0"), ("#AE3D56", "#EE8DA2")]
HEADER = "<!-- {what}. Drawn by scripts/make-brand.py; edit that, not this file. -->"


def pages(dark: bool) -> str:
    return "".join(f'<path fill="{a[1 if dark else 0]}" d="{rrect(x, y, 18, 24, 3)}"/>'
                   for (x, y), a in zip([(12, 6), (34, 6), (12, 34), (34, 34)], ACCENTS))


def mark_file(dark: bool) -> str:
    return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" role="img"'
            f' aria-label="{LABEL}">\n{HEADER.format(what=LABEL + " mark")}\n<title>{LABEL}</title>\n{pages(dark)}\n</svg>\n')


def wordmark_file(dark: bool) -> str:
    """The mark as tall as the ascenders, then the name, on one baseline (base at y 150,
    ascenders at y 10), as the suite's wordmark."""
    paths, width = set_name(NAME)
    base = 150.0
    d = " ".join(flip(p, base) for p in paths)
    k = (base - 10) / 52
    tx, ty = -12 * k, 10 - 6 * k
    name_x = 40 * k + 40
    height = base - DESC_END + 10 if any(ch in NAME for ch in "gpy") else 162
    ink = INK_DARK if dark else INK
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {fmt(name_x + width)} {fmt(height)}"'
            f' role="img" aria-label="{LABEL}">\n{HEADER.format(what=LABEL + " wordmark")}\n<title>{LABEL}</title>\n'
            f'<g transform="translate({fmt(tx)} {fmt(ty)}) scale({k:.4f})">{pages(dark)}</g>\n'
            f'<path transform="translate({fmt(name_x)} 0)" fill="none" stroke="{ink}"'
            f' stroke-width="{fmt(W)}" d="{d}"/>\n</svg>\n')


def favicon_file() -> str:
    def small(dark: bool) -> str:
        return "".join(f'<path fill="{a[1 if dark else 0]}" d="{rrect(x, y, 6, 6.5, 1)}"/>'
                       for (x, y), a in zip([(1.5, 1), (8.5, 1), (1.5, 8.5), (8.5, 8.5)], ACCENTS))
    return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">\n'
            f'{HEADER.format(what=LABEL + " favicon")}\n'
            '<style>.d{display:none}@media (prefers-color-scheme:dark){.l{display:none}.d{display:inline}}</style>\n'
            f'<g class="l">{small(False)}</g><g class="d">{small(True)}</g>\n</svg>\n')


def outputs() -> dict[Path, str]:
    brand = ROOT / "site" / "assets" / "brand"
    return {
        brand / "mark.svg": mark_file(False),
        brand / "mark-dark.svg": mark_file(True),
        brand / "wordmark.svg": wordmark_file(False),
        brand / "wordmark-dark.svg": wordmark_file(True),
        ROOT / "site" / "favicon.svg": favicon_file(),
        ROOT / "assets" / "brand" / "launcher-wordmark.svg": wordmark_file(False),
        ROOT / "assets" / "brand" / "launcher-wordmark-dark.svg": wordmark_file(True),
    }


def main(argv: list[str]) -> int:
    check = "--check" in argv
    stale = []
    for path, text in outputs().items():
        current = path.read_text() if path.exists() else None
        if current == text:
            continue
        if check:
            stale.append(path.relative_to(ROOT))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
            print(f"make-brand: wrote {path.relative_to(ROOT)}")
    if stale:
        print("make-brand: out of date: " + ", ".join(map(str, stale)) + " (run scripts/make-brand.py)")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
