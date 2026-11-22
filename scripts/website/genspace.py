#!/usr/bin/env python3
"""Draw the images of the space page, `website/space/`: ink splats and the star field.

Usage, from the repo root: `python3 scripts/website/genspace.py`.

Writes into `website/space/img/` splat-{colour}.svg (an ink splat with rays and droplets,
one shape per colour, fixed seeds), stars.svg (a tile of stars in a few sizes and colours),
nebula.svg (soft blobs of colour for behind the stars) and salminis.svg, a still tile of the
salminis, little astronaut bots in glass helmets (an original design), for readers without
JavaScript or who ask for less motion; and `website/space/salminis.js`, their species for
the swimming shoal. The hero title is set over splat-pink.svg, so a change of its colour is
checked by cdk/tests/test_website_pages.py. The art generators live here, outside
`website/`, so they never deploy; the output is committed and CI doesn't rerun them
(.stories/10-website/spec.md "Variants" (Art generators) and issue 06).
"""
import math, random
from pathlib import Path

SPACE = Path(__file__).resolve().parents[2] / "website" / "space"
out = SPACE / "img"
COLOURS = {"pink": "#ff2fa0", "lime": "#b6ff2e", "cyan": "#29e6ff", "orange": "#ff8a1f",
           "violet": "#9b6bff"}

def smooth(points):
    """A closed path through the midpoints of the points, curving at each point."""
    n = len(points)
    mids = [((points[i][0] + points[(i+1) % n][0]) / 2, (points[i][1] + points[(i+1) % n][1]) / 2)
            for i in range(n)]
    d = f"M{mids[-1][0]:.1f} {mids[-1][1]:.1f}"
    for i in range(n):
        d += f"Q{points[i][0]:.1f} {points[i][1]:.1f} {mids[i][0]:.1f} {mids[i][1]:.1f}"
    return d + "Z"

def splat(seed):
    rng = random.Random(seed)
    cx = cy = 100
    pts, drops = [], []
    n = 28
    rays = set(rng.sample(range(n), 6))
    for i in range(n):
        a = 2 * math.pi * i / n + rng.uniform(-0.06, 0.06)
        r = 44 + rng.uniform(-7, 9)
        if i in rays:   # a ray of ink, ending in a droplet
            ray = r + rng.uniform(16, 28)
            pts.append((cx + math.cos(a - 0.05) * (r + 4), cy + math.sin(a - 0.05) * (r + 4)))
            pts.append((cx + math.cos(a) * ray, cy + math.sin(a) * ray))
            pts.append((cx + math.cos(a + 0.05) * (r + 4), cy + math.sin(a + 0.05) * (r + 4)))
            dr = ray + rng.uniform(6, 10)
            drops.append((cx + math.cos(a) * dr, cy + math.sin(a) * dr, rng.uniform(3, 5)))
        else:
            pts.append((cx + math.cos(a) * r, cy + math.sin(a) * r))
    for _ in range(7):  # loose droplets around
        a, dr = rng.uniform(0, 2 * math.pi), rng.uniform(60, 88)
        drops.append((cx + math.cos(a) * dr, cy + math.sin(a) * dr, rng.uniform(1.5, 4.5)))
    circles = "".join(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r:.1f}"/>' for x, y, r in drops)
    return f'<path d="{smooth(pts)}"/>{circles}'

for i, (name, colour) in enumerate(COLOURS.items()):
    open(f"{out}/splat-{name}.svg", "w").write(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200"><g fill="{colour}">{splat(i + 5)}</g></svg>\n')

rng = random.Random(42)
stars = []
for _ in range(140):
    x, y = rng.uniform(0, 400), rng.uniform(0, 400)
    r = rng.choice([0.6, 0.6, 0.8, 1, 1.4])
    c = rng.choice(["#ffffff", "#ffffff", "#ffd6f0", "#d6fbff", "#f0ffd0"])
    stars.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r}" fill="{c}" fill-opacity="{rng.choice([0.4, 0.7, 1])}"/>')
for _ in range(6):   # a few twinkles: four-pointed stars
    x, y, s = rng.uniform(10, 390), rng.uniform(10, 390), rng.uniform(3, 5)
    stars.append(f'<path d="M{x:.1f} {y-s:.1f}L{x+s/4:.1f} {y:.1f}L{x:.1f} {y+s:.1f}L{x-s/4:.1f} {y:.1f}Z'
                 f'M{x-s:.1f} {y:.1f}L{x:.1f} {y+s/4:.1f}L{x+s:.1f} {y:.1f}L{x:.1f} {y-s/4:.1f}Z" fill="#ffffff"/>')
open(f"{out}/stars.svg", "w").write(
    f'<svg xmlns="http://www.w3.org/2000/svg" width="400" height="400" viewBox="0 0 400 400">{"".join(stars)}</svg>\n')

blobs = []
for colour, (x, y, r) in zip(["#ff2fa0", "#29e6ff", "#9b6bff", "#b6ff2e"],
                             [(150, 220, 160), (620, 120, 140), (520, 560, 200), (120, 700, 120)]):
    blobs.append(f'<circle cx="{x}" cy="{y}" r="{r}" fill="{colour}" fill-opacity="0.12"/>')
open(f"{out}/nebula.svg", "w").write(
    '<svg xmlns="http://www.w3.org/2000/svg" width="800" height="900" viewBox="0 0 800 900">'
    '<defs><filter id="f" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="50"/></filter></defs>'
    f'<g filter="url(#f)">{"".join(blobs)}</g></svg>\n')

# Salminis: little astronaut bots in glass helmets, in a bold cartoon style. An original
# design: five species in soft colours with friendly faces on their screens and a
# jetpack flame behind. (They were space salmon once; the shoal kept the name.) The markup goes to
# salminis.js for the shoal that swims about (shoal.js), and into salminis.svg, a still
# tile for readers without JavaScript or who ask for less motion.
import json
LINE = 'stroke="#1a0b2e" stroke-width="2.2" stroke-linejoin="round"'
INK = "#1a0b2e"

def eye_open(cx, cy, r=2.6):
    return (f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="#9ff6ff"/>'
            f'<circle cx="{cx+0.9}" cy="{cy-0.9}" r="0.9" fill="#ffffff"/>')

def eye_happy(cx, cy):
    return f'<path d="M{cx-2.6} {cy+1}Q{cx} {cy-2.6} {cx+2.6} {cy+1}" stroke="#9ff6ff" stroke-width="1.8" fill="none" stroke-linecap="round"/>'

def heart(cx, cy):
    return (f'<path d="M{cx} {cy+2.4}L{cx-2.6} {cy-0.2}Q{cx-3.2} {cy-2.8} {cx-1.2} {cy-2.8}Q{cx} {cy-2.8} {cx} {cy-1.4}'
            f'Q{cx} {cy-2.8} {cx+1.2} {cy-2.8}Q{cx+3.2} {cy-2.8} {cx+2.6} {cy-0.2}Z" fill="#ff7ac0"/>')

SMILE = '<path d="M34 13.5Q37.5 16.5 41 13.5" stroke="#9ff6ff" stroke-width="1.6" fill="none" stroke-linecap="round"/>'

# The faces on each bot's screen, all friendly: eyes at (33, 9) and (42, 9).
FACES = {
    "wink":   eye_open(33, 9) + eye_happy(42, 9.5) + SMILE,
    "happy":  eye_happy(33, 9.5) + eye_happy(42, 9.5) + SMILE,
    "heart":  heart(33, 9) + heart(42, 9) + SMILE,
    "sleepy": ('<path d="M30.6 9.5H35.4M39.6 9.5H44.4" stroke="#9ff6ff" stroke-width="1.8" stroke-linecap="round"/>'
               '<circle cx="37.5" cy="14.4" r="1.4" fill="#9ff6ff"/>'),
    "star":   eye_open(33, 9, 2.9) + eye_open(42, 9, 2.9) + SMILE,
}

def bot(body, light, dark, face, extras="", helmet="plain"):
    """One astronaut bot, viewBox 0 0 74 52, facing right like the shoal swims. Its jetpack
    flame is its own group (class "tail"), so it flickers the way the salmon's tail wagged."""
    flame = ('<g class="tail"><path d="M12 25Q2 21 -2 27Q2 33 12 29Z" fill="#ffb347" '
             f'{LINE.replace("2.2", "1.4")}/><path d="M12 26Q6 25 3 27Q6 29 12 28Z" fill="#fff3a0"/></g>')
    pack = f'<rect x="10" y="17" width="8" height="16" rx="2.5" fill="#a9b3d6" {LINE}/>'
    legs = (f'<rect x="22" y="33" width="5" height="6" rx="2" fill="{dark}" {LINE}/>'
            f'<rect x="31" y="33" width="5" height="6" rx="2" fill="{dark}" {LINE}/>')
    torso = (f'<rect x="17" y="18" width="24" height="17" rx="7" fill="{body}" {LINE}/>'
             f'<rect x="23" y="22" width="12" height="7" rx="3" fill="{light}"/>'
             '<circle cx="26.5" cy="25.5" r="1.4" fill="#ff7ac0"/><circle cx="31.5" cy="25.5" r="1.4" fill="#29e6ff"/>')
    arm = (f'<path d="M39 26Q45 27 49 23" stroke="{INK}" stroke-width="5.2" fill="none" stroke-linecap="round"/>'
           f'<path d="M39 26Q45 27 49 23" stroke="{body}" stroke-width="2.6" fill="none" stroke-linecap="round"/>'
           f'<circle cx="50" cy="22" r="2.8" fill="{light}" {LINE.replace("2.2", "1.6")}/>')
    head = (f'<rect x="27" y="1" width="21" height="18" rx="7" fill="{body}" {LINE}/>'
            f'<rect x="29.5" y="3.5" width="16" height="13" rx="5" fill="{INK}"/>'
            '<circle cx="30" cy="16.5" r="1.6" fill="#ff9ac8" fill-opacity="0.9"/>'
            '<circle cx="45" cy="16.5" r="1.6" fill="#ff9ac8" fill-opacity="0.9"/>')
    glass = ('<circle cx="37.5" cy="10" r="14" fill="#29e6ff" fill-opacity="0.12" stroke="#29e6ff" stroke-width="2"/>'
             '<path d="M28 3Q32 -1.5 38 -2" stroke="#ffffff" stroke-width="2" fill="none" stroke-linecap="round" stroke-opacity="0.85"/>')
    if helmet == "cracked":
        glass += '<path d="M47 0L44.5 4L47 6L44 9" stroke="#ffffff" stroke-width="1" fill="none" stroke-opacity="0.9"/>'
    if helmet == "antenna":
        glass += ('<path d="M37.5 -4L39.5 -11" stroke="#1a0b2e" stroke-width="1.6"/>'
                  '<path d="M39.8 -15.5L41 -12.7L44 -12.5L41.7 -10.6L42.4 -7.7L39.8 -9.3L37.2 -7.7L37.9 -10.6L35.6 -12.5L38.6 -12.7Z" '
                  'fill="#ffd23f" stroke="#1a0b2e" stroke-width="1"/>')
    # The head and its helmet sit centred over the body.
    return ('<g transform="translate(4 8)">' + flame + pack + legs + torso + arm
            + '<g transform="translate(-8.5 0)">' + head + FACES[face] + glass + '</g>' + extras + '</g>')

# A sticker on the chest, and a star badge on the shoulder.
BANDAID = ('<g transform="rotate(-20 37 30)"><rect x="33.5" y="28.5" width="7" height="3" rx="1" '
           f'fill="#ffd8a8" {LINE.replace("2.2", "1")}/><circle cx="35.8" cy="30" r="0.45" fill="#c8553d"/>'
           '<circle cx="38.2" cy="30" r="0.45" fill="#c8553d"/></g>')
BADGE = '<path d="M20.5 19L21.5 21.2L23.9 21.4L22 23L22.6 25.3L20.5 24L18.4 25.3L19 23L17.1 21.4L19.5 21.2Z" fill="#ffd23f" stroke="#1a0b2e" stroke-width="0.8"/>'

# Each species: its look, its size (times 74 px), how fast it cruises (px/s), how fast its
# flame flickers (s per swing), how many fly at once. The names are the shoal's, from when
# its crew were salmon.
SPECIES = {
    "mini":    dict(svg=bot("#9fe8c8", "#e4fff3", "#4fae8a", "wink"),
                    scale=0.55, speed=70, wag=0.22, count=5),
    "salmini": dict(svg=bot("#ffb3a1", "#ffe6dc", "#d9776a", "happy", BANDAID),
                    scale=0.9, speed=38, wag=0.42, count=4),
    "pink":    dict(svg=bot("#ffa6cf", "#ffe1ef", "#d0679a", "heart"),
                    scale=0.9, speed=44, wag=0.38, count=3),
    "brute":   dict(svg=bot("#b8b3ff", "#ebe9ff", "#7a72d6", "sleepy", BADGE, helmet="cracked"),
                    scale=1.45, speed=22, wag=0.7, count=2),
    "boss":    dict(svg=bot("#ffd27a", "#fff1cf", "#d39a2e", "star", BADGE, helmet="antenna"),
                    scale=3.2, speed=16, wag=1.0, count=1),
}
VIEW = "0 0 74 52"

data = {name: {k: v for k, v in sp.items() if k != "svg"} | {"svg": sp["svg"]} for name, sp in SPECIES.items()}
open(SPACE / "salminis.js", "w").write(
    "// Generated by scripts/website/genspace.py: the salmini species for shoal.js.\n"
    f"window.SALMINIS = {{ view: {json.dumps(VIEW)}, species: {json.dumps(data, indent=2)} }};\n")

# The still tile, for readers without JavaScript or who ask for less motion: a few of each,
# the boss left out.
rng = random.Random(9)
W, H = 640, 720
defs = "".join(f'<symbol id="{n}" viewBox="{VIEW}">{sp["svg"]}</symbol>' for n, sp in SPECIES.items() if n != "boss")
kinds = ["mini", "mini", "salmini", "salmini", "pink", "pink", "brute", "mini", "salmini"]
placed, tries = [], 0
while len(placed) < len(kinds) and tries < 5000:
    tries += 1
    x, y = rng.uniform(20, W - 130), rng.uniform(20, H - 90)
    if all(((x - px) ** 2 + (y - py) ** 2) ** 0.5 > 140 for px, py, *_ in placed):
        placed.append((x, y, rng.uniform(-18, 6), rng.choice([0.55, 0.7, 0.85])))
uses = []
for (x, y, r, o), kind in zip(placed, kinds):
    sc = SPECIES[kind]["scale"]
    w, h = 74 * sc, 52 * sc
    uses.append(f'<g transform="translate({x:.0f} {y:.0f}) rotate({r:.0f} {w/2:.0f} {h/2:.0f})" opacity="{o}">'
                f'<use href="#{kind}" width="{w:.0f}" height="{h:.0f}"/></g>')
open(f"{out}/salminis.svg", "w").write(
    f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">'
    f'<defs>{defs}</defs>{"".join(uses)}</svg>\n')
