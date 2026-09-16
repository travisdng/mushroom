#!/usr/bin/env python3
"""
Generate Mushroom's icon set from hand-drawn pixel grids.

The 16x16 grid is the canonical drawing and matches the one rendered in the UI
by src/components/common/Icon.tsx. 24x24 is drawn separately because 16 -> 24
is a 1.5x scale, which would smear the pixel structure; every other size is an
integer multiple of 16 and is scaled with nearest-neighbour so the pixels stay
square.

Run:  python tools/make_icons.py
"""

from pathlib import Path

from PIL import Image

OUT = Path(__file__).resolve().parent.parent / "src-tauri" / "icons"

PALETTE = {
    "o": (0, 0, 0, 255),        # outline
    "r": (156, 31, 31, 255),    # cap
    "R": (199, 59, 59, 255),    # cap highlight
    "c": (242, 227, 194, 255),  # stem / spots
    "C": (201, 177, 137, 255),  # stem shadow
    ".": (0, 0, 0, 0),          # transparent
}

GRID_16 = [
    "................",
    ".....oooooo.....",
    "...ooRRRRRRoo...",
    "..oRRRrrccrRRo..",
    ".oRRrrrrccrrrRo.",
    ".oRrrccrrrrrrro.",
    "oRrrrccrrrccrrRo",
    "orrrrrrrrrccrrro",
    ".oooooooooooooo.",
    "....occccccco...",
    "....ocCccccCo...",
    "....ocCccccCo...",
    "....ocCcccCCo...",
    "....occccccco...",
    ".....ooooooo....",
    "................",
]

GRID_24 = [
    "........................",
    "........oooooooo........",
    "......ooRRRRRRRRoo......",
    ".....oRRRRRRRRRRRRo.....",
    "....oRRRRRccccRRRRRo....",
    "...oRRRRRRccccRRRRRRo...",
    "...oRRRccRRRRRRccRRRo...",
    "..oRRRRccRRRRRRccRRRRo..",
    "..oRrrrrrrrrrrrrrrrrrRo.",
    ".oRrrrrccrrrrrrrccrrrrRo",
    ".orrrrrccrrrrrrrccrrrrro",
    ".orrrrrrrrrrrrrrrrrrrrro",
    "..oooooooooooooooooooo..",
    ".....occcccccccccco.....",
    ".....ocCcccccccccCo.....",
    ".....ocCcccccccccCo.....",
    ".....ocCcccccccccCo.....",
    ".....ocCccccccccCCo.....",
    ".....ocCccccccccCCo.....",
    ".....occcccccccccco.....",
    "......oooooooooooo......",
    "........................",
    "........................",
    "........................",
]


def render(grid):
    """Turn a character grid into an RGBA image, one character per pixel."""
    size = len(grid)
    for y, row in enumerate(grid):
        if len(row) != size:
            raise SystemExit(f"row {y} is {len(row)} chars, expected {size}")

    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    px = img.load()
    for y, row in enumerate(grid):
        for x, ch in enumerate(row):
            colour = PALETTE.get(ch)
            if colour is None:
                raise SystemExit(f"unknown palette character {ch!r} at {x},{y}")
            px[x, y] = colour
    return img


def scale(img, size):
    """Nearest-neighbour only — this is pixel art, never interpolate it."""
    return img.resize((size, size), Image.Resampling.NEAREST)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    base16 = render(GRID_16)
    base24 = render(GRID_24)

    def at(size):
        if size == 24:
            return base24
        if size % 16 != 0:
            # Not an integer multiple of the grid: step up from 256 instead of
            # smearing the 16px drawing.
            return scale(base16, 256).resize(
                (size, size), Image.Resampling.NEAREST
            )
        return scale(base16, size)

    # Sizes Tauri's bundler and Windows both expect.
    named = {
        "32x32.png": 32,
        "128x128.png": 128,
        "128x128@2x.png": 256,
        "icon.png": 512,
    }
    for name, size in named.items():
        at(size).save(OUT / name)

    for size in (16, 24, 32, 48, 64, 128, 256):
        at(size).save(OUT / f"{size}x{size}.png")

    # Windows Store / MSIX logos, kept consistent with the rest.
    for name, size in {
        "Square30x30Logo.png": 30,
        "Square44x44Logo.png": 44,
        "Square71x71Logo.png": 71,
        "Square89x89Logo.png": 89,
        "Square107x107Logo.png": 107,
        "Square142x142Logo.png": 142,
        "Square150x150Logo.png": 150,
        "Square284x284Logo.png": 284,
        "Square310x310Logo.png": 310,
        "StoreLogo.png": 50,
    }.items():
        at(size).save(OUT / name)

    # Multi-resolution .ico. The base must be the LARGEST image: Pillow drops
    # any requested size bigger than the base. append_images then supplies each
    # smaller size exactly as drawn, so nothing gets resampled.
    ico_sizes = [16, 24, 32, 48, 64, 128, 256]
    images = {s: at(s) for s in ico_sizes}
    images[256].save(
        OUT / "icon.ico",
        format="ICO",
        sizes=[(s, s) for s in ico_sizes],
        append_images=[images[s] for s in ico_sizes if s != 256],
    )

    print(f"wrote {len(list(OUT.glob('*')))} files to {OUT}")


if __name__ == "__main__":
    main()
