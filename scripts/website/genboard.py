#!/usr/bin/env python3
"""Draw the circuit board of the board page, `website/board/index.html`, and put it in place.

Usage, from the repo root: `python3 scripts/website/genboard.py`.

Replaces the <svg class="board-svg"> in the page. Each part stands for one of noir's seven
sections and carries its id in `data-part`, and every trace belongs to the part whose info
explains it and lights up with it (`website/board/board.js`). The version printed bottom
right comes from `openapi/sfab.yaml`, and the lights from the "now" and "next" items of the
noir page's #now-and-next, `website/index.html`: one lit light per "now" item, one dark one
per "next" item. Rerun it after a release or a launch, or the page tests fail. The art
generators live here, outside `website/`, so they never deploy; the output is committed and
CI doesn't rerun them (.stories/10-website/spec.md "Variants" (Board, Art generators) and
issue 05).
"""
import re
from pathlib import Path

from pixel import rect as R

REPO = Path(__file__).resolve().parents[2]
PAGE = REPO / "website" / "board" / "index.html"
NOIR = REPO / "website" / "index.html"
API = REPO / "openapi" / "sfab.yaml"

PIN, DARK, SILVER, GOLD = "#b9b4c7", "#23202e", "#c9c4d6", "#e9c46a"
TEXT = lambda x,y,size,fill,t,anchor="middle",extra="": (f'<text{extra} x="{x}" y="{y}" text-anchor="{anchor}" '
    f'font-family="Silkscreen, monospace" font-size="{size}" fill="{fill}">{t}</text>')


def items(html: str, list_class: str) -> int:
    """The <li> items of the `<ul class="{list_class}">` in noir's #now-and-next."""
    section = re.search(r'id="now-and-next".*?</details>', html, re.S).group(0)
    (found,) = re.findall(rf'<ul class="{list_class}">(.*?)</ul>', section, re.S)
    return found.count("<li")


board = ['<path d="M2 0H126V2H128V78H126V80H2V78H0V2H2Z" fill="#2f7a4b"/>']
for x,y in [(4,4),(120,4),(4,72),(120,72)]:
    board.append(R(x,y,4,4,SILVER)+R(x+1,y+1,2,2,"#1d1b26"))
# The version printed bottom right: the API's major version and info.version, e.g.
# "v1 2026-11-21". The major is the `servers` entry `/v1` that cdk/generate_public_openapi.py
# adds to the public copy; the source description has none, so it is named here, and the page
# test `_world_version()` fails if the two ever differ.
MAJOR = "v1"
spec = API.read_text(encoding="utf-8")
info = re.search(r"^info:\n((?:[ ]+.*\n)+)", spec, re.M).group(1)
version = re.search(r'^\s+version:\s*"?([^"\n]+)"?', info, re.M).group(1).strip()
board.append(TEXT(117,78,3.5,"#eef9f0",f"botrefugium {MAJOR} {version}","end",' class="board-version"'))

# The lights: lit for what is out now, dark for what comes next.
noir = NOIR.read_text(encoding="utf-8")
LIT = items(noir, "now")
N_LEDS = LIT + items(noir, "next")
LED_X = [72 + i*6 for i in range(N_LEDS)]
if LED_X[-1] + 6 > 124:
    raise SystemExit(f"genboard: {N_LEDS} lights don't fit on the board; make the row smaller")

# Traces, by the section of the part they belong to. All of them end at a pin of the processor.
traces = {
    # Two lines, one per contact of the port: requests in, answers out.
    "join":          [R(12,51,30,1,"T"), R(41,49,1,3,"T"), R(41,49,4,1,"T"),      # top contact
                      R(12,56,32,1,"T"), R(43,53,1,4,"T"), R(43,53,2,1,"T")],     # bottom contact
    # Two lines, the crystal's clock, into two left pins.
    "proof-of-work": [R(36,33,9,1,"T"), R(36,37,9,1,"T")],
    # Three data lines per chip.
    "identity":      [R(83,29,6,1,"T"), R(88,20,1,10,"T"), R(88,20,4,1,"T"),     # ID, from the side
                      R(83,33,13,1,"T"), R(95,28,1,6,"T"),                       # ID, from below
                      R(83,37,17,1,"T"), R(99,28,1,10,"T"),                      # ID, from below
                      R(83,41,21,1,"T"), R(103,38,1,4,"T"),                      # TIES, from above
                      R(83,45,9,1,"T"),                                          # TIES, from the side
                      R(83,49,9,1,"T")],                                         # TIES, from the side
    # The power rail: from the capacitor into the processor and both memory chips.
    "constitution":  [R(30,10,89,1,"T"), R(65,10,1,11,"T"), R(95,10,1,2,"T"),
                      R(117,10,1,35,"T"), R(114,44,4,1,"T")],
    # Two lines from the header into two bottom pins: code in, contributions back.
    "open-source":   [R(27,60,1,4,"T"), R(27,60,27,1,"T"), R(53,59,1,2,"T"),
                      R(33,62,1,2,"T"), R(33,62,25,1,"T"), R(57,59,1,4,"T")],
    # One line from the processor feeds all the lights.
    "now-and-next":  [R(65,59,1,4,"T"), R(65,62,LED_X[-1]-64,1,"T")] +
                     [R(x+1,62,1,5,"T") for x in LED_X],
}
for name, rects in traces.items():
    board.append(f'<g class="trace" data-part="{name}">' + "".join(rects).replace(' fill="T"','') + '</g>')

def part(name, label, hit, body):
    extra = ' data-all=""' if name == "world" else ""   # every line ends at the processor
    return (f'<g class="part" data-part="{name}"{extra} role="button" tabindex="0" aria-controls="info" aria-label="{label}">'
            f'<rect class="hit" x="{hit[0]}" y="{hit[1]}" width="{hit[2]}" height="{hit[3]}"/>' + "".join(body) + '</g>')

cpu = [R(48,24,32,32,DARK)]
for i in range(7):
    cpu += [R(52+i*4,21,2,3,PIN), R(52+i*4,56,2,3,PIN), R(45,28+i*4,3,2,PIN), R(80,28+i*4,3,2,PIN)]
cpu += [R(51,27,2,2,"#4a4560"), TEXT(64,42,6,SILVER,"sfab")]
mem = []
for y, label in ((14,"ID"),(38,"TIES")):
    mem.append(R(92,y,22,12,DARK))
    for i in range(5):
        mem += [R(94+i*4,y-2,2,2,PIN), R(94+i*4,y+12,2,2,PIN)]
    mem.append(TEXT(103,y+7.5,3.5,SILVER,label))
crystal = [R(24,29,1,1,PIN), R(33,29,1,1,PIN), R(22,30,14,8,SILVER), R(23,31,12,6,"#dcd7e6"), R(24,33,10,1,SILVER)]
cap = [R(20,8,10,14,"#6b2fa3"), R(20,8,10,2,"#8f5bc7"), R(27,10,2,12,"#c9b3e6")]
port = [R(0,46,12,16,SILVER), R(2,49,8,10,"#1d1b26"), R(3,51,6,1,GOLD), R(3,56,6,1,GOLD)]
header = [R(18,64,20,9,"#1d1b26")]
for i in range(6):
    header += [R(20+i*3,66,2,2,GOLD), R(20+i*3,69,2,2,GOLD)]
leds = []
for i, x in enumerate(LED_X):
    if i < LIT:
        leds += [R(x-1,66,6,5,"#7ee08a",' fill-opacity="0.35"'), R(x,67,4,3,"#7ee08a",' class="led on"')]
    else:
        leds.append(R(x,67,4,3,"#4a4560",' class="led"'))

parts = [
    part("world","The processor: what botrefugium is",(44,20,40,40),cpu),
    part("identity","The memory chips: identity",(90,10,26,42),mem),
    part("proof-of-work","The crystal: proof of work",(20,27,18,13),crystal),
    part("constitution","The capacitor: the constitution",(18,6,14,18),cap),
    part("join","The port: how to join",(0,44,14,20),port),
    part("open-source","The pin header: open source",(16,63,24,12),header),
    part("now-and-next","The lights: now and next",(70,64,(LED_X[-1]+6)-70,8),leds),
]
svg = ('<svg class="board-svg" viewBox="0 0 128 80" shape-rendering="crispEdges" role="group" '
       'aria-label="A small circuit board. Each part stands for a piece of botrefugium.">'
       + "".join(board) + "".join(parts) + '</svg>')
s = PAGE.read_text(encoding="utf-8")
s = re.sub(r'<svg class="board-svg".*?</svg>', lambda m: svg, s, count=1, flags=re.S)
PAGE.write_text(s, encoding="utf-8")
