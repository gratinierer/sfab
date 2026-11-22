# Build both versions' design and frame

Blocked by: 2-identity-api#12

`spec.md` "Design and content". The hosting is story 2's issue 12; this ticket replaces the lorem ipsum in
`website/` with the real frame, still with placeholder text.

## Scope

- `website/index.html` and `website/agents/index.html`, one page each, with the five sections in order and the
  same anchors in both: `#what`, `#join`, `#now-and-next`, `#proof-of-work`, `#open-source`.
- The header: the wordmark alone, which already includes the shelter icon, with `alt="botrefugium"`, and the switch link, "I'm an agent →" on `/` and "← human version"
  on `/agents/`.
- The logo files already in the repo, `website/shelter-icon.svg` and `website/shelter-wordmark.svg`, with the icon
  as the SVG favicon.
- The human CSS: pixel frames, the logo's palette, Silkscreen headings from a `woff2` (Regular, Latin subset) taken once from
  Fontsource's `@fontsource/silkscreen` at a pinned version and committed with its `OFL.txt` in `website/fonts/`,
  the version and source noted here under `## Comments`, a system body font, and light and dark through `prefers-color-scheme`.
- The agent CSS: always dark, a monospace system font, cyan and rust accents, faint scan lines and grain in CSS
  only, terminal touches, and nothing that moves.
- One column on narrow screens, `<title>`, `description`, `alt` text, no JavaScript and nothing external.

## Done when

- Both pages pass an HTML validator, load no file from another host, and contain no `<script>`.
- Scan lines and grain stay within one CSS variable capping their opacity at 8 %, and each text colour meets WCAG AA
  (4.5:1, or 3:1 for large text) in both versions and both colour schemes, checked with a tool against the plain
  background and against that background under the full 8 % overlay.
- The switch link and a `<details>` block work by keyboard, and the pages read well at 360 px wide.
- On a deploy: both versions and the favicon appear at `<domain_name>`.

## Comments

- 2026-11-21 (claude-opus-5-5, implementing): the font is `@fontsource/silkscreen@5.3.0` from the npm registry
  (`npm pack`, tarball shasum `418edaecef16338e086d7374d3209f297f130b3b`): `files/silkscreen-latin-400-normal.woff2`
  (sha256 `e6c72ea4702249202bcdd79d3343057e4e25ef1f04e3fcffd8602ab53b40b4cc`) as
  `website/fonts/silkscreen-latin-400-normal.woff2`, and the package's `LICENSE`, the SIL Open Font License 1.1 for
  Silkscreen, as `website/fonts/OFL.txt`.
- 2026-11-21 (claude-opus-5-5, implementing): `cdk/tests/test_website_pages.py` checks the built pages in
  `scripts/check`: sections and anchors, header, favicon, title and description, `alt`, `<details>`, no `<script>`
  or `on*` handler, every loaded file local, well-formed nesting, one column within 360 px, and WCAG AA (4.5:1 for every
  `--fg-*` colour) in both versions and both human schemes, plain and with `--bg` blended with the overlay's
  `--scan` and `--grain` at `--overlay-max` (0.08). The lowest ratio is the agent version's rust under the grain,
  5.07:1. The built pages passed `html-validate@11.21.1` (`html-validate:recommended`) and the W3C Nu validator
  with no errors or warnings, run once by hand; neither is in `scripts/check`, which would add a Node or Java
  dependency. Left: keyboard use of the switch link and `<details>` and the 360 px reading in a real browser, and the
  deploy check.
