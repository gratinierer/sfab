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
- `website/style.css`, the old page's stylesheet, is deleted, and so are the tests and test helpers that only
  checked it. `assemble.js` is not brought in. The board ticket, 05,
  ports what board needs from the scratch site's copies into `/board/`.
- `scripts/website/genbot.py`, with only the sets the site uses (`-noir` here, the light lilac set in 05), its
  preview-only sets (`-dark`, `-cream`, `bot-tile-many*`) dropped.
- In `#now-and-next`, "next" becomes a `<ul>` like "now", one `<li>` per coming story, with the same words, so
  the meter, the board's LEDs and their tests count items instead of parsing a sentence.
- A page test checks that noir's `hud-meter` ("2 of 8 features are live") matches the "now" and "now plus next"
  item counts in its `#now-and-next`, so the owner's hand edits there keep the meter true.
- `scripts/website/gennoir.py`, rewritten to write into `website/index.html`, with its output committed.
- The tests find sections by `id`, not by element: noir's are `<details class="panel">`, board's `.part-info`
  blocks (05) and space's `section.stop` (06). `test_each_version_has_the_five_sections_in_order_with_the_same_anchors`
  reads each page's section ids that way, and `lambda/core/tests/worked_example.rs` slices noir's
  `#proof-of-work` panel by its id up to its matching `</details>`, counting nested `<details>` so the example's
  own closing tag doesn't end the slice, and finds exactly one `<details>` nested in the panel, the example,
  instead of `find("<section id=\"proof-of-work\"")` and one `<details>` per section.
- `cdk/tests/test_website_pages.py` changes these tests: `test_the_header_holds_the_wordmark_and_one_switch_link_to_the_other_version`
  takes a link home named botrefugium, the wordmark image or text, and until 07 noir keeps one switch link,
  "I'm an agent →", in its header, so `/` is never without a way to the agent version;
  `test_the_human_version_has_a_light_and_a_dark_scheme` gives way to one fixed scheme per page; and
  `test_the_human_headings_use_the_committed_silkscreen...` checks noir, then board in 05, but not space; and
  `test_the_layout_is_one_column_that_fits_a_360_px_phone` allows a grid that is one column at 360 px, as
  `spec.md` "Variants" (Layout) says, instead of rejecting every `columns:`.
- `cdk/tests/test_website_pages.py` follows the amended rules for `/`: one fixed dark scheme, motion only under
  `prefers-reduced-motion: no-preference`, a link home named botrefugium in the header, and the five anchors in
  order with extras allowed.
- `test_every_text_colour_meets_wcag_aa_plain_and_under_the_full_overlay` and
  `test_text_colours_are_only_set_through_the_checked_variables` read that table instead of `--bg` and `--fg-*`:
  every `color:` must name a text variable the table lists for that page.
- The contrast test's table of text and background variable pairs, as `spec.md` "Variants" (Colour schemes)
  says, gains noir's `--ink`, `--muted`, `--link` and `--signal` on `--night` and `--panel`, every pair the stylesheet uses.

## Done when

- `lambda/core/tests/worked_example.rs` finds its values in the noir `website/index.html` and in
  `website/agents/index.html`.
- The page tests pass for `/` and `/agents/`, including no script on either, WCAG AA contrast against noir's
  dark scheme, and no animation outside the reduced-motion guard. No animation flashes more than 3 times a
  second.
- No `{{` or lorem ipsum is left after the generate step, and no committed page holds an address.
