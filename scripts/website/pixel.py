"""Pixel-art helpers the website's art generators share (gennoir.py, genboard.py)."""


def rect(x, y, w, h, fill, extra=""):
    """One SVG rect, `extra` being any further attributes, written as ` name="value"`."""
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{fill}"{extra}/>'


def grid_rects(grid, colours, ox=0, oy=0):
    """One 1x1 rect per character of `grid` that `colours` maps to a fill, offset by `ox`, `oy`."""
    out = []
    for y, row in enumerate(grid.strip("\n").splitlines()):
        for x, c in enumerate(row):
            if c in colours:
                out.append(rect(ox + x, oy + y, 1, 1, colours[c]))
    return "".join(out)
