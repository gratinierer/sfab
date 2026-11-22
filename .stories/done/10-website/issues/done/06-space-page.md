# Port the space page to `/space/`

Blocked by: 04

`spec.md` "Variants". The source is `/workspaces/.scratch/website/space/`.

## Scope

- `website/space/index.html`, `space.css`, `space.js`, `shoal.js`, `salminis.js` and `img/`.
- Noir's seven sections, ids and order, with noir's paragraphs and lists word for word. Only headings may keep
  themed prefixes, so "Humans don't land here" becomes noir's sentence.
- Dark as its one fixed scheme. The splats and the salmini shoal move only under
  `prefers-reduced-motion: no-preference`, and the cards show at once instead of waiting at `opacity: 0` for
  their splat to land, as `spec.md` "Variants" (Motion) says; without the scripts, or with reduced motion, the page shows its still
  tile and every section.
- Its `#proof-of-work` links to `/#proof-of-work` for the worked example.
- The section helpers from 04 find this page's sections by `id`.
- `scripts/website/genspace.py`, with its output committed.
- The contrast test's table of text and background variable pairs, as `spec.md` "Variants" (Colour schemes)
  says, gains space's `--ink` and `--muted` on `--card` and `--space`, every pair the stylesheet uses, and the hero title's `--ink` against each fill of `splat-pink.svg`.

## Done when

- The page tests pass for `/space/`: scripts only from `/space/`, the seven anchors in order, WCAG AA contrast
  against its dark scheme, with text on solid fills or checked against the SVG beneath it, and motion only under the reduced-motion guard, CSS or
  script, with no card hidden or faded while its splat lands.
- With scripts off, every section is readable and nothing is hidden.
