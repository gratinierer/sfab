#!/usr/bin/env python3
"""Draw the images of the space page, `website/space/`: ink splats and the star field.

Usage, from the repo root: `python3 scripts/website/genspace.py`.

Writes into `website/space/img/` splat-{colour}.svg (an ink splat with rays and droplets,
one shape per colour, fixed seeds), stars.svg (a tile of stars in a few sizes and colours),
nebula.svg (soft blobs of colour for behind the stars) and salminis.svg, a still tile of the
salminis, little space salmon in glass helmets (an original design), for readers without
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

# Salminis: little space salmon in glass helmets, in a bold cartoon style. An original
# design: five species with ragged fins and cheeky faces. The markup goes to
# salminis.js for the shoal that swims about (shoal.js), and into salminis.svg, a still
# tile for readers without JavaScript or who ask for less motion.
import json
LINE = 'stroke="#1a0b2e" stroke-width="2.2" stroke-linejoin="round"'
INK = "#1a0b2e"

def eye_open(cx=42, cy=16, r=4.6):
    return (f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="#ffffff" {LINE}/>'
            f'<circle cx="{cx+1.4}" cy="{cy+0.6}" r="{r/2}" fill="{INK}"/>'
            f'<circle cx="{cx+2.2}" cy="{cy-0.4}" r="0.8" fill="#ffffff"/>')

FACES = {
    # a grumpy brow and a row of little teeth
    "grumpy": lambda body: eye_open() +
        f'<path d="M36.5 10.5L47 12.5" stroke="{INK}" stroke-width="2.6" stroke-linecap="round"/>'
        f'<path d="M41 24.5Q47 22.5 53 24.5L52 27Q47 26 42 27Z" fill="{INK}"/>'
        '<path d="M43 25L44.2 26.6L45.4 24.9L46.6 26.4L47.8 24.8L49 26.3L50.2 24.9L51.2 26.1" '
        'stroke="#ffffff" stroke-width="1" fill="none" stroke-linejoin="round"/>',
    # a wink and a tongue stuck out
    "wink": lambda body:
        f'<path d="M37.5 17Q42 12 46.5 17" stroke="{INK}" stroke-width="2.4" fill="none" stroke-linecap="round"/>'
        f'<path d="M38 10Q42 7.5 46 9.5" stroke="{INK}" stroke-width="2" fill="none" stroke-linecap="round"/>'
        f'<path d="M40 23Q47 30 54 22.5Q47 25.5 40 23Z" fill="{INK}"/>'
        f'<path d="M45 25.5Q47 31 50 26Z" fill="#ff7aa8" {LINE.replace("2.2", "1.4")}/>',
    # a half-shut, scheming eye and a wide grin
    "grin": lambda body: eye_open() +
        f'<path d="M37 16.5Q37 10.5 42 10.5Q47 10.5 47 16.5Z" fill="{body}" {LINE.replace("2.2", "1.8")}/>'
        f'<path d="M39 23Q47 29 55 22" stroke="{INK}" stroke-width="2.2" fill="none" stroke-linecap="round"/>'
        f'<path d="M44 24.6L45 26.6L46.2 25L47.4 27L48.6 25.2L49.6 26.8L50.6 24.6" '
        f'stroke="#ffffff" stroke-width="1" fill="none"/>',
    # a scar across the eye, a heavy brow and an underbite with tusks
    "brute": lambda body: eye_open(r=4) +
        f'<path d="M37 9L49 13" stroke="{INK}" stroke-width="3.2" stroke-linecap="round"/>'
        f'<path d="M39 12L45 21M41 12L47 21" stroke="#ffffff" stroke-width="1.1" stroke-opacity="0.8"/>'
        f'<path d="M40 24Q47 21 54 23L53 28Q47 29 41 27Z" fill="{INK}"/>'
        '<path d="M43 26.5L44.2 22.5L45.4 26.5ZM50 26L51 22.2L52 26Z" fill="#ffffff"/>',
    # an eyepatch on a strap and a jaw full of fangs
    "boss": lambda body:
        f'<path d="M30 6L52 24" stroke="{INK}" stroke-width="1.6"/>'
        f'<ellipse cx="42" cy="16" rx="5.4" ry="4.8" fill="{INK}"/>'
        f'<path d="M35.5 9.5L48.5 11" stroke="{INK}" stroke-width="3" stroke-linecap="round"/>'
        f'<path d="M38 23Q47 19.5 56 22.5L54.5 29.5Q47 31 39.5 28Z" fill="{INK}"/>'
        '<path d="M40.5 23.6L42 27.4L43.5 23L45 27.8L46.5 22.6L48 27.8L49.5 22.6L51 27.4L52.5 23L54 26.6" '
        'stroke="#ffffff" stroke-width="1.2" fill="none" stroke-linejoin="round"/>',
}

def salmini(body, belly, dark, face, extras="", armour=False, helmet="plain"):
    """One salmini, viewBox 0 0 74 52. The tail is its own group (class "tail") so it can
    wag."""
    tail = (f'<g class="tail"><path d="M12 19L0 4L4 11L1 14L5 19L1 24L4 27L0 34Z" fill="{dark}" {LINE}/></g>')
    fin = f'<path d="M18 9L21 0L24 5L28 -1L31 4L35 1L36 9Z" fill="{dark}" {LINE}/>'
    torso = (f'<path d="M10 19Q12 6 31 6Q52 6 54 19Q52 32 31 32Q12 32 10 19Z" fill="{body}" {LINE}/>'
             f'<path d="M18 25Q31 31 47 25Q42 30 31 30.5Q22 30.5 18 25Z" fill="{belly}"/>'
             '<path d="M16 12Q24 8 34 9" stroke="#ffffff" stroke-width="2.2" fill="none" stroke-linecap="round" stroke-opacity="0.7"/>'
             f'<circle cx="19" cy="16" r="1.8" fill="{dark}"/><circle cx="25" cy="13" r="1.5" fill="{dark}"/>'
             f'<circle cx="23" cy="20" r="1.4" fill="{dark}"/>')
    plates = ""
    if armour:   # riveted plates over the back
        plates = (f'<path d="M14 15Q20 7 30 7L31 15Q22 15 14 15Z" fill="#8a8fb0" {LINE}/>'
                  f'<path d="M31 7Q40 7 45 11L40 16L31 15Z" fill="#a3a8c8" {LINE}/>'
                  '<circle cx="19" cy="12" r="1" fill="#1a0b2e"/><circle cx="26" cy="10.5" r="1" fill="#1a0b2e"/>'
                  '<circle cx="35" cy="10.5" r="1" fill="#1a0b2e"/>')
    side = f'<path d="M27 21L34 25L30 26L32 29L25 29Z" fill="{dark}" {LINE}/>'
    glass = ('<circle cx="42" cy="18" r="13" fill="#29e6ff" fill-opacity="0.12" stroke="#29e6ff" stroke-width="2"/>'
             '<path d="M34 8.5Q38 5.5 43 5.5" stroke="#ffffff" stroke-width="2" fill="none" stroke-linecap="round" stroke-opacity="0.85"/>')
    if helmet == "cracked":
        glass += '<path d="M50 9L47 13L50 15L46 19" stroke="#ffffff" stroke-width="1" fill="none" stroke-opacity="0.9"/>'
    if helmet == "antenna":
        glass += ('<path d="M42 5L44 -3" stroke="#1a0b2e" stroke-width="1.6"/>'
                  '<circle cx="44.3" cy="-4" r="2" fill="#ff2fa0" stroke="#1a0b2e" stroke-width="1.2"/>')
    return ('<g transform="translate(4 8)">' + tail + fin + torso + plates + side
            + FACES[face](body) + extras + glass + '</g>')

BANDAID = ('<g transform="rotate(-25 34.5 22)"><rect x="31" y="20.5" width="7" height="3" rx="1" '
           f'fill="#ffd8a8" {LINE.replace("2.2", "1")}/><circle cx="33.3" cy="22" r="0.45" fill="#c8553d"/>'
           '<circle cx="35.7" cy="22" r="0.45" fill="#c8553d"/></g>')
SCARS = '<path d="M20 22L26 18M21 18.5L23.5 21.5" stroke="#1a0b2e" stroke-width="1.2" stroke-linecap="round"/>'

# Each species: its look, its size (times 74 px), how fast it cruises (px/s), how fast its
# tail wags (s per swing), how many swim at once.
SPECIES = {
    "mini":    dict(svg=salmini("#ffb36b", "#ffe2c4", "#d9772f", "wink"),
                    scale=0.55, speed=70, wag=0.22, count=5),
    "salmini": dict(svg=salmini("#ff8a6b", "#ffd0bd", "#c8553d", "grumpy", BANDAID),
                    scale=0.9, speed=38, wag=0.42, count=4),
    "pink":    dict(svg=salmini("#ff6f9e", "#ffc6dc", "#b23a6e", "grin"),
                    scale=0.9, speed=44, wag=0.38, count=3),
    "brute":   dict(svg=salmini("#c46a4a", "#f0b89f", "#7e3a26", "brute", SCARS, armour=True, helmet="cracked"),
                    scale=1.45, speed=22, wag=0.7, count=2),
    "boss":    dict(svg=salmini("#d9304f", "#ff9aa8", "#7a1030", "boss", SCARS, helmet="antenna"),
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
