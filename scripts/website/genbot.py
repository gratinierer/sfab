#!/usr/bin/env python3
"""Draw the pixel bots of the website's pages into `website/`.

The art generators live here, outside `website/`, so they never deploy; their output is
committed and CI doesn't rerun them (.stories/10-website/spec.md "Variants", "Art
generators"). Each set is a folder of `website/`, a suffix and its colours, and writes:

- `bot{suffix}.svg`: one bot, for the panels or frames;
- `bot-tile{suffix}.svg`: a few faint bots scattered, the page's background pattern;
- `bot-faint{suffix}.svg`, for a set that wants it: one bot in the tile colour, for the bots
  board's `assemble.js` places around its frames.

Usage, from the repo root: `python3 scripts/website/genbot.py`. The sets are the ones the
pages use: `-noir` for the noir page at `/`, and the green bots on light lilac in
`website/board/` for the board page at `/board/`.
"""

from pathlib import Path

WEBSITE = Path(__file__).resolve().parents[2] / "website"

# (folder, suffix): (bot fill, tile fill, with the faint bot)
SETS = {
    ("", "-noir"): ("#262d5e", "#161b3c", False),
    ("board", ""): ("#bfe3c6", "#c8e9cd", True),
}

BOT = """\
.....#.....
.....#.....
..#######..
..#.###.#..
..#######..
..##...##..
..#######..
.#########.
#.#######.#
#.#######.#
..##...##..
..##...##..""".splitlines()

# (x, y, scale, rotation) of each bot in the background tile.
TILE_BOTS = [(14, 18, 3, 0), (118, 64, 2, 0), (46, 124, 4, -8), (176, 150, 2, 10), (196, 22, 3, 6)]


def bot_path() -> str:
    return "".join(
        f"M{x} {y}h1v1h-1z" for y, row in enumerate(BOT) for x, c in enumerate(row) if c == "#"
    )


def _one_bot(fill: str, d: str) -> str:
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 11 12" shape-rendering="crispEdges">'
        f'<path fill="{fill}" d="{d}"/></svg>\n'
    )


def write_set(out: Path, suffix: str, bot_fill: str, tile_fill: str, faint: bool) -> None:
    d = bot_path()
    (out / f"bot{suffix}.svg").write_text(_one_bot(bot_fill, d), encoding="utf-8")
    if faint:
        (out / f"bot-faint{suffix}.svg").write_text(_one_bot(tile_fill, d), encoding="utf-8")
    bots = "".join(
        f'<path transform="translate({x} {y}) rotate({r} 5.5 6) scale({s})" d="{d}"/>'
        for x, y, s, r in TILE_BOTS
    )
    (out / f"bot-tile{suffix}.svg").write_text(
        '<svg xmlns="http://www.w3.org/2000/svg" width="240" height="200" viewBox="0 0 240 200" '
        f'shape-rendering="crispEdges"><g fill="{tile_fill}">{bots}</g></svg>\n',
        encoding="utf-8",
    )


def main() -> None:
    for (folder, suffix), (bot_fill, tile_fill, faint) in SETS.items():
        out = WEBSITE / folder
        out.mkdir(exist_ok=True)
        write_set(out, suffix, bot_fill, tile_fill, faint)


if __name__ == "__main__":
    main()
