#!/usr/bin/env python3
"""Rasterise packaging/icons/forge3d.svg's geometry into PNG, ICO and ICNS (needs Pillow).

The SVG is the source of truth; this script mirrors its few shapes so the build needs no SVG
renderer. Output files are committed, so releases do not depend on running this.
Usage: python3 scripts/release/gen-icons.py
"""
import os
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "..", "packaging", "icons")
BG, TOP, LEFT, RIGHT = "#20242c", "#8fc7ff", "#3f7fd0", "#2a5799"
SHAPES = [
    (TOP, [(512, 176), (802, 344), (512, 512), (222, 344)]),
    (LEFT, [(222, 344), (512, 512), (512, 848), (222, 680)]),
    (RIGHT, [(802, 344), (512, 512), (512, 848), (802, 680)]),
]


def render(size):
    ss = 4  # supersample
    n = 1024 * ss
    img = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([0, 0, n - 1, n - 1], radius=224 * ss, fill=BG)
    for colour, pts in SHAPES:
        d.polygon([(x * ss, y * ss) for x, y in pts], fill=colour)
    return img.resize((size, size), Image.LANCZOS)


def main():
    os.makedirs(OUT, exist_ok=True)
    for s in (16, 32, 48, 64, 128, 256, 512, 1024):
        render(s).save(os.path.join(OUT, f"forge3d-{s}.png"))
    big = render(1024)
    big.save(os.path.join(OUT, "forge3d.ico"), sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    big.save(os.path.join(OUT, "forge3d.icns"))


if __name__ == "__main__":
    main()
