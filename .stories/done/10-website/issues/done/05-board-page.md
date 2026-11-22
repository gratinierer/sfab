# Port the board page to `/board/`

Blocked by: 04

`spec.md` "Variants", "Board". The source is `/workspaces/.scratch/website/board/`.

## Scope

- `website/board/index.html`, `board.css` and `board.js`. What board took from the old page's `style.css`,
  `assemble.js` and the bot tiles moves into `website/board/`, so the page loads nothing from the site root but
  the icon, the wordmark and the font.
- The info blocks take noir's seven section ids, in noir's order, with noir's paragraphs and lists word for word.
  Only the headings keep their themed prefixes. `board.js` writes those ids into the URL instead of part names,
  and with no hash it shows `#world`. Without the script every block is listed.
- Each part keeps the section its scratch heading names: the processor (`cpu`) `#world`, the port `#join`, the
  memory chips `#identity`, the capacitor `#constitution`, the crystal `#proof-of-work`, the lights (`leds`)
  `#now-and-next` and the pin header `#open-source`. `data-part` on `.part`, `.trace` and `.part-info`,
  `board.js`'s selectors and hash, and `genboard.py`'s trace keys and `data-all` move to those ids, and the
  `.part-info` blocks follow noir's order.
- Light lilac as its one fixed scheme. Scroll assembly only under `prefers-reduced-motion: no-preference`, and
  `assemble.js` builds only each frame's border and bots: its content shows at once, as `spec.md` "Variants"
  (Motion) says.
- Its `#proof-of-work` links to `/#proof-of-work` for the worked example.
- `scripts/website/genboard.py`, with its output committed, reading `openapi/sfab.yaml` relative to the repo
  instead of `/workspaces/sfa/`, and taking its lit and total LEDs from the "now" and "next" `<li>` items, `ul.now li` and `ul.next li`, in
  noir's `#now-and-next` (04) instead of the fixed `N_LEDS, LIT = 8, 2`.
- A page test checks that the API version printed on the board matches `info.version` in `openapi/sfab.yaml`,
  and that the lit and total LEDs match the "now" and "now plus next" item counts, so a release or a launch
  that isn't regenerated fails the check.
- The section helpers from 04 find this page's sections by `id`.
- The page tests allow scripts on board, but only from `/board/`.
- The contrast test's table of text and background variable pairs, as `spec.md` "Variants" (Colour schemes)
  says, gains board's text variables on its lilac background and frames, every pair the stylesheet uses.

## Done when

- The page tests pass for `/board/`: no script outside `/board/`, the seven anchors in order, WCAG AA contrast
  against its lilac scheme, and motion only under the reduced-motion guard, CSS or
  script, with no text or list hidden, faded or half built at any scroll position.
- With scripts off, the page shows every block. With them on, `/board/` opens on `#world` and `/board/#join` on
  `#join`, and a click or Enter on a part updates the URL with that section's id.
