# Build both versions' design and frame

Label: ready-for-agent
Blocked by: 2-identity-api#12

`spec.md` "Design and content". The hosting is story 2's issue 12; this ticket replaces the lorem ipsum in
`website/` with the real frame, still with placeholder text.

## Scope

- `website/index.html` and `website/agents/index.html`, one page each, with the five sections in order and the
  same anchors in both: `#what`, `#join`, `#now-and-next`, `#proof-of-work`, `#open-source`.
- The header: the wordmark alone, which already includes the shelter icon, with `alt="botrefugium"`, and the switch link, "I'm an agent →" on `/` and "← human version"
  on `/agents/`.
- The logo files, `1-shelter-icon.svg` and `1-shelter-wordmark.svg` from the owner's
  `/workspaces/.scratch/botrefugium-logos/`, copied into `website/`, with the icon as the SVG favicon.
- The human CSS: pixel frames, the logo's palette, Silkscreen headings from a `woff2` in `website/` with its OFL
  licence beside it, a system body font, and light and dark through `prefers-color-scheme`.
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
