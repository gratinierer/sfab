# Port the space page to `/space/`

Label: ready-for-agent
Blocked by: 04

`spec.md` "Variants". The source is `/workspaces/.scratch/website/space/`.

## Scope

- `website/space/index.html`, `space.css`, `space.js`, `shoal.js`, `salminis.js` and `img/`.
- Noir's seven sections, ids and order, with noir's paragraphs and lists word for word. Only headings may keep
  themed prefixes, so "Humans don't land here" becomes noir's sentence.
- Dark as its one fixed scheme. The splats and the salmini shoal move only under
  `prefers-reduced-motion: no-preference`; without the scripts, or with reduced motion, the page shows its still
  tile and every section.
- Its `#proof-of-work` links to `/#proof-of-work` for the worked example.
- The section helpers from 04 find this page's sections by `id`.
- `scripts/website/genspace.py`, with its output committed.
- The contrast test's table of text and background variable pairs, as `spec.md` "Variants" (Colour schemes)
  says, gains space's `--ink` and `--muted` on `--card` and `--space`, every pair the stylesheet uses.

## Done when

- The page tests pass for `/space/`: scripts only from `/space/`, the seven anchors in order, WCAG AA contrast
  against its dark scheme, with text only on solid fills over the nebula and stars, and motion only under the reduced-motion guard.
- With scripts off, every section is readable and nothing is hidden.
