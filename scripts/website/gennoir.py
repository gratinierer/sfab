#!/usr/bin/env python3
"""Draw the pixel art of the noir page, `website/index.html`, and put it in place.

Usage, from the repo root: `python3 scripts/website/gennoir.py`.

Fills <svg class="city"> with the night city: sky, moon, a searchlight projecting the
botrefugium icon, buildings with lit windows (fixed seed) and the caped bot on a roof.
Fills each <svg class="icon" data-icon="..."> with its pixel icon. The art generators live
here, outside `website/`, so they never deploy; the output is committed and CI doesn't rerun
them (.stories/10-website/spec.md "Variants", "Art generators").
"""
import random
import re
from pathlib import Path

from pixel import grid_rects, rect

PAGE = Path(__file__).resolve().parents[2] / "website" / "index.html"

# --- the city -------------------------------------------------------------------------
W, H = 256, 112
rng = random.Random(3)
sky = [rect(0,0,W,H,"#0b0d1a"), rect(0,40,W,30,"#10132b"), rect(0,70,W,42,"#161a3a")]
for y in (39, 69):                       # dithered band edges
    sky += [rect(x, y, 1, 1, "#10132b" if y == 39 else "#161a3a") for x in range(0, W, 2)]
stars = [rect(rng.randrange(W), rng.randrange(36), 1, 1, "#e8ecff", f' fill-opacity="{rng.choice([0.4,0.7,1])}"') for _ in range(40)]
moon = []
for y in range(-8, 9):
    for x in range(-8, 9):
        if x*x + y*y <= 64:
            moon.append(rect(214+x, 20+y, 1, 1, "#f4e9b8"))
moon += [rect(210,17,2,2,"#e0d39a"), rect(217,23,3,2,"#e0d39a"), rect(215,15,1,1,"#e0d39a")]
clouds = [rect(150,26,40,3,"#1e2347"), rect(158,23,22,3,"#1e2347"), rect(30,14,34,3,"#1a1f40"), rect(38,11,16,3,"#1a1f40")]
# The searchlight: a beam from a rooftop to the sky, the icon in a pale disc where it lands.
beam = ('<g class="beam">'
        '<polygon points="62,74 66,74 128,16 104,10" fill="#ffd23f" fill-opacity="0.16"/>'
        '<ellipse cx="116" cy="13" rx="15" ry="10" fill="#ffd23f" fill-opacity="0.85"/>'
        '<image href="/shelter-icon.svg" x="108" y="5" width="16" height="16" style="image-rendering:pixelated"/>'
        '</g>')
buildings, windows = [], []
x = 0
while x < W:
    w = rng.choice([14, 18, 22, 26, 30])
    h = rng.randint(28, 62)
    top = H - h
    colour = rng.choice(["#0f1226", "#13173a", "#0d1022"])
    buildings.append(rect(x, top, w, h, colour))
    if rng.random() < 0.4:               # antenna
        buildings.append(rect(x + w // 2, top - 6, 1, 6, colour))
        windows.append(rect(x + w // 2, top - 7, 1, 1, "#ff5470", ' class="blink"'))
    for wy in range(top + 4, H - 4, 5):
        for wx in range(x + 3, x + w - 3, 4):
            r = rng.random()
            if r < 0.22:
                windows.append(rect(wx, wy, 2, 2, "#ffd23f"))
            elif r < 0.28:
                windows.append(rect(wx, wy, 2, 2, "#6ad0ff", ' class="flicker"' if rng.random() < 0.3 else ""))
    x += w + rng.choice([0, 1, 2])
# The searchlight's roof and the hero's roof, in front of the skyline.
front = [rect(52,74,22,38,"#090b17"), rect(58,70,10,4,"#090b17"), rect(60,72,6,2,"#ffd23f"),
         rect(150,82,40,30,"#090b17"), rect(148,80,44,2,"#1e2347")]
HERO = """
......AA......
.....BBBB.....
....BBBBBB....
....BVVVVB....
....BVEVEB....
....BBBBBB....
...CBBBBBBC...
..CCBBDDBBCC..
.CCCBBBBBBCCC.
CCCCBBBBBBCCCC
CCC.BBDDBB.CCC
CC..BB..BB..CC
C...BB..BB...C
....BB..BB....
...BBB..BBB...
"""
hero = grid_rects(HERO, {"A": "#6ad0ff", "B": "#2fa8ff", "V": "#0b0d1a", "E": "#e8ecff",
                         "C": "#2a2550", "D": "#1b6fd1"}, 163, 65)
city = ('<svg class="city" viewBox="0 0 256 112" shape-rendering="crispEdges" role="img" '
        'aria-label="A night city. A searchlight projects the botrefugium sign onto the clouds; '
        'a small caped robot watches from a rooftop.">'
        + "".join(sky + stars + moon + clouds) + beam + "".join(buildings + windows + front) + hero + '</svg>')

# --- the icons ------------------------------------------------------------------------
ICONS = {
    "globe": """
..#####..
.#.#.#.#.
#..#.#..#
#########
#..#.#..#
#########
#..#.#..#
.#.#.#.#.
..#####..""",
    "door": """
..#####..
.#.....#.
.#.....#.
.#.....#.
.#...#.#.
.#.....#.
.#.....#.
.#.....#.
#########""",
    "mask": """
.........
.........
#.......#
#########
#..###..#
#########
.###.###.
.........
.........""",
    "scroll": """
.#######.
#.......#
.#######.
.#.....#.
.#.###.#.
.#.....#.
.#.###.#.
.#######.
#.......#""",
    "hourglass": """
#########
.#.....#.
..#...#..
...#.#...
....#....
...#.#...
..#.#.#..
.#.###.#.
#########""",
    "friends": """
.##...##.
#..#.#..#
#..#.#..#
.##...##.
.........
.##...##.
#..#.#..#
#..#.#..#
#..#.#..#""",
    "house": """
....#....
...#.#...
..#...#..
.#.....#.
#########
.#.....#.
.#.##..#.
.#.##..#.
.#######.""",
    "book": """
.........
###...###
#..#.#..#
#.##.##.#
#..#.#..#
#.##.##.#
#..#.#..#
###.#.###
.........""",
    "screen": """
#########
#.......#
#.#...#.#
#..#.#..#
#...#...#
#.......#
#########
..#...#..
.#.....#.""",
    "code": """
.........
..#...#..
.#.....#.
#...#...#
.#.....#.
..#...#..
.........
.........
.........""",
}
def icon(name):
    return (f'<svg class="icon" data-icon="{name}" viewBox="0 0 9 9" shape-rendering="crispEdges" aria-hidden="true">'
            + grid_rects(ICONS[name], {"#": "currentColor"}) + '</svg>')

s = PAGE.read_text(encoding="utf-8")
s = re.sub(r'<svg class="city".*?</svg>', lambda m: city, s, count=1, flags=re.S)
s = re.sub(r'<svg class="icon" data-icon="(\w+)".*?</svg>', lambda m: icon(m.group(1)), s, flags=re.S)
PAGE.write_text(s, encoding="utf-8")
