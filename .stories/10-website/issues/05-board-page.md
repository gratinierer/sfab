# Port the board page to `/board/`

Label: ready-for-agent
Blocked by: 04

`spec.md` "Variants", "Board". The source is `/workspaces/.scratch/website/board/`.

## Scope

- `website/board/index.html`, `board.css` and `board.js`. What board took from the old page's `style.css`,
  `assemble.js` and the bot tiles moves into `website/board/`, so the page loads nothing from the site root but
  the icon, the wordmark and the font.
- The info blocks take noir's seven section ids, in noir's order, with noir's paragraphs and lists word for word.
  Only the headings keep their themed prefixes. `board.js` writes those ids into the URL instead of part names,
  and with no hash it shows `#world`. Without the script every block is listed.
- Light lilac as its one fixed scheme. Scroll assembly only under `prefers-reduced-motion: no-preference`.
- Its `#proof-of-work` links to `/#proof-of-work` for the worked example.
- `scripts/website/genboard.py`, with its output committed.
- The page tests allow scripts on board, but only from `/board/`.

## Done when

- The page tests pass for `/board/`: no script outside `/board/`, the seven anchors in order, WCAG AA contrast
  against its lilac scheme, and motion only under the reduced-motion guard.
- With scripts off, the page shows every block. With them on, `/board/` opens on `#world` and `/board/#join` on
  `#join`, and a click or Enter on a part updates the URL with that section's id.
