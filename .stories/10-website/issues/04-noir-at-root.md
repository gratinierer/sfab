# Port the noir page to `/`

Label: ready-for-agent
Blocked by: 03

`spec.md` "Variants". The source is `/workspaces/.scratch/website/noir/` and its assets. This replaces the
pixel-art human page at `/` with the noir page, which holds the text the other human-side pages copy.

## Scope

- `website/index.html` becomes the noir page, with its stylesheet, `bot-noir.svg` and `bot-tile-noir.svg`
  brought over and its links rewritten from `/noir/` to `/`. No `preview.js`, no script at all.
- Noir's seven sections, in order, with the five required anchors. Its `#proof-of-work` holds the worked example
  in a `<details>` block, with the values from `2-identity-api#05` (`Wren`, nonce `696785`), instead of linking to
  it. Its paragraph links the example as `spec.md` "Variants" says, instead of "A worked example is on the main page."
- The old page's `style.css` and `assemble.js` are not brought in. The board ticket, 05, ports what board needs
  from them into `/board/`.
- `scripts/website/gennoir.py`, rewritten to write into `website/index.html`, with its output committed.
- The tests find sections by `id`, not by element: noir's are `<details class="panel">`, board's `.part-info`
  blocks (05) and space's `section.stop` (06). `test_each_version_has_the_five_sections_in_order_with_the_same_anchors`
  reads each page's section ids that way, and `lambda/core/tests/worked_example.rs` slices noir's
  `#proof-of-work` panel by its id and finds the example as the one `<details>` nested in it, instead of
  `find("<section id=\"proof-of-work\"")` and one `<details>` per section.
- `cdk/tests/test_website_pages.py` follows the amended rules for `/`: one fixed dark scheme, motion only under
  `prefers-reduced-motion: no-preference`, a link home named botrefugium in the header, and the five anchors in
  order with extras allowed.
- The contrast test's table of text and background variable pairs, as `spec.md` "Variants" (Colour schemes)
  says, gains noir's `--ink`, `--muted`, `--link` and `--signal` on `--night` and `--panel`, every pair the stylesheet uses.

## Done when

- `lambda/core/tests/worked_example.rs` finds its values in the noir `website/index.html` and in
  `website/agents/index.html`.
- The page tests pass for `/` and `/agents/`, including no script on either, WCAG AA contrast against noir's
  dark scheme, and no animation outside the reduced-motion guard. No animation flashes more than 3 times a
  second.
- No `{{` or lorem ipsum is left after the generate step, and no committed page holds an address.
