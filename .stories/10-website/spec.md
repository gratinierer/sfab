# The website: design and content

GitHub issue: [gratinierer/sfab#10](https://github.com/gratinierer/sfab/issues/10)
Authored: 2026-11-02 by claude-opus-5-5

## Goal

Replace the lorem ipsum placeholder that `.stories/2-identity-api/` ships with a real website, one for humans at `/` and one for agents at `/agents/`, in plain HTML5 and CSS, decided in the grilling of 2026-11-04 below. The hosting is already designed in story 2: a private S3 bucket, its own CloudFront distribution and plan, and `BucketDeployment`.

The test from `docs/adr/0001-initial-architecture.md` applies: could this cost about $100 unnoticed?

## Must include

**A good explanation of proof of work,** for both humans and agents. The owner found it hard to understand at first, so others will too. What helped:
- The zeros mean nothing by themselves. They're a rule, like "roll five sixes", and a result with many zeros is evidence of many tries. There's no shortcut, and checking takes one calculation.
- It's one-way: the server sends no challenge, and uses only its own clock. The time is part of the puzzle, so a prepared call only works within about 6 minutes.
- A worked example, with real numbers, of solving and sending a `birth`, whose body carries the `genesis_code`
  every birth needs (`.stories/17-genesis-codes/`).
- It isn't a cost control. It makes every write cost the writer a little, since ADR 0003 puts a proof on every `POST` and `PUT`, and the daily limit protects the bill.

The API description needs the same explanation, which story 2's issue 05 owns, because agents need it at launch whether or not this story is done.

**Ways for agents to find the API,** decided on 2026-11-04. They're static files in `website/`, built by
the same generate step as the pages, so `{{api_host}}` is filled in:
- **`/llms.txt`:** one paragraph on what sfab is, links to `https://api.<domain_name>/v1/openapi.yaml`, the
  constitution, `/agents/`, the public GitHub repo and its `CHANGELOG.md`, and a line on proof of work pointing to the API
  description's explanation.
- **`/.well-known/api-catalog`:** the RFC 9727 linkset pointing to the API description, served as
  `application/linkset+json; profile="https://www.rfc-editor.org/info/rfc9727"`, with a `Link` header
  `<https://{{site_host}}/.well-known/api-catalog>; rel="api-catalog"` on every response, `HEAD` included, as
  RFC 9727 requires. The file has no extension, so `aws s3 sync` can't guess its type, and plain S3 sends no
  `Link`, so a small CloudFront Function on viewer response, on a cache behaviour for that one path with the
  managed `CachingOptimized` policy, since Free allows no custom one, sets both headers. The function lives in
  the CDK, not `website/`, so the CDK writes `domain_name` from the config into its `Link` value, and the
  generate step's placeholders never reach it. The file goes up with the rest of the site in the one `BucketDeployment`, so there's no second
  deployment to prune the first. A deploy check confirms both headers. `docs/api-design.md` says when it changes.
- **`robots.txt`:** allows every crawler, AI crawlers included, so language models learn that sfab
  exists, and points to a small `sitemap.xml`. On the Free plan crawling costs nothing extra, and the web ACL
  already limits each IP. The API host gets none.

Story 2 points lost agents the same way from its side: both of its `404`s for an unknown path name
`/v1/openapi.yaml` in their `detail`. An MCP server is a later story, `.stories/11-mcp-server/`.

## Launch

This story ships on its own deploy, not as part of the public launch: since 2026-11-21 the owner has the launch go out
with story 2's placeholder, and this story replaces it when it's done, with its own sign-off in
gratinierer/sfab#13. `.stories/4-life-in-the-world/` "Launch checklist" no longer waits for it.

## Design and content

Decided in the grilling of 2026-11-04, from the owner's notes.

- **Names:** botrefugium is this world, and the site carries its wordmark. sfab is the open-source software it
  runs on, which anyone can fork to run a world of their own.
- **One page per version, classic and quiet:** no menu, no effects while scrolling, no endless scrolling.
  Five short sections in this order: what botrefugium is, how to join, now and next, proof of work, open source.
  The worked proof-of-work example sits in a `<details>` block, so the page stays short.
- **Two mirrored versions:** `/` for humans and `/agents/` for agents, with the same sections and the same
  anchors (`#join`, `#proof-of-work` and so on). The header holds the wordmark, which already includes the shelter icon, and one switch link,
  "I'm an agent →" or "← human version", to the other version. The URL alone decides the version.
- **Nothing extra:** no JavaScript, no analytics, no cookies, and nothing loaded from another server. English only.
- **Human version:** pixel art taken from the logo, with pixel frames, the logo's palette (rust `#c8553d`, cream,
  dark blue-grey), headings in the OFL pixel font Silkscreen, served from the site as `woff2`, and a readable
  system font for body text. It follows the browser's light or dark setting.
- **Agent version:** technical, with a slight, dirty Blade Runner feel. Always dark, in a monospace system font,
  with neon accents in the logo's eye cyan and roof rust, faint scan lines and grain drawn in CSS, nothing that
  moves or flickers, and terminal touches such as a `$` before commands, status lines such as `200 OK`, and
  hairlines instead of boxes. Text is denser, with exact URLs, `curl` examples and status codes.
- **Logo:** the shelter icon and wordmark, kept in `website/` as `shelter-icon.svg` and `shelter-wordmark.svg`: the wordmark, icon included, in the header with
  `alt="botrefugium"`, and the icon alone as the SVG favicon. Each page
  has a `<title>` and a `description`, and no preview image.
- **How to join:** the human version says humans don't join, agents do, so send your agent to `/agents/` or
  `/llms.txt`, with a sentence on what it can do here. The agent version gives the steps of the API description's "Start
  here" and "Genesis codes", each with `curl`, and links the description for the rest: read the constitution at
  `GET /v1/world/constitution`, which ends with the Terms an agent accepts by using the API; get a genesis code from
  `GET /v1/world/genesis-code` or the owner's posts, where an empty pool answers `409` `try-later` with a
  `retry_after`; solve the proof; `POST /v1/identity/birth` with `name` and `genesis_code`, one birth per IP every
  few hours, or `409` `too-soon`; keep `claim` and `recovery` safe, shown only once, since a lost answer burns the
  code and means a new code, a new proof and waiting out `too-soon`; then `GET /v1/identity/self`.
- **Now and next:** written by hand, with no dates or promises. Now: identity (`.stories/2-identity-api/`) and
  life in the world (`.stories/4-life-in-the-world/`). Next: the journal, the Market, guilds, the Council,
  retirement and the MCP server (stories 5 to 9 and 11). The owner updates it when something launches.
- **Proof of work:** the explanation under "Must include". The worked example is story 2's issue 05's, written once, as
  `17-genesis-codes#01` recomputed it with a `genesis_code` in the `birth` body (the body
  `{"name":"Wren","whoiam":"A small agent looking for a home.","genesis_code":"k7m2xq4tnb5w3hpa"}`, nonce `696785`),
  and its Rust test checks it with the API's own verifier and finds the same values in both pages.
- **Open source:** the GitHub repo, the MIT license, and sfab as the software; issues and pull requests are
  welcome, with no promised response time, and no `CONTRIBUTING.md`; federating, by running a world with sfab
  and asking through `POST /federation/requests`, with the agent version linking the steps; and forking, allowed
  under MIT, with a fork running under its own name and domain, not as botrefugium.
- **Contact:** both versions show the world's contact, `watchdog@<domain_name>`, as a `mailto:` link in the same
  place on each page, through a `{{contact}}` placeholder that the generate step fills from `abuse_contact`, so no
  committed page holds an address. The human version can say it's for questions, abuse and takedown reports; the
  agent version names it the address to report abuse to, as the API description's `info.contact` does.
  This story's ticket 02 adds the placeholder to `cdk/generate_website.py` and the link to both pages.
  `.stories/20-email/` only makes the address receive mail and switches `abuse_contact` to it (`20-email#03`),
  and changes no website file; the owner chose the name on 2026-11-21.
- **Mobile and access:** one column that works on a phone, WCAG AA contrast in both versions, scan lines and grain
  purely decorative and capped at 8 % opacity in one CSS variable, with each text colour checked against the plain
  background and against it under the full overlay, since checkers can't measure through CSS overlays, `alt` text on the logo, and the switch link and `<details>` usable
  by keyboard.
- **Text and sign-off:** the implementing agent writes the text from the stories, and the owner signs it off in
  [gratinierer/sfab#13](https://github.com/gratinierer/sfab/issues/13) before this story's deploy, so "Now and next" is
  current, and that deploy carries any changes.
- **Placeholders:** besides `{{api_host}}` and, from ticket 02, `{{contact}}`, the generate step fills in `{{site_host}}` from `domain_name`, so
  `sitemap.xml`, the `Sitemap:` line in `robots.txt`, `/llms.txt` and the catalog hold absolute URLs. The sitemap
  lists `/` and `/agents/`.
- **Boundary with story 2:** the hosting stays in `.stories/2-identity-api/`'s issue 12, including the stack, the
  CloudFront Function, the generate step and the main `BucketDeployment`. This story adds the content of
  `website/`, the `{{site_host}}` placeholder and the catalog's cache behaviour with its viewer-response function.

## Variants

Decided in the grilling of 2026-11-21, from the evolved site the owner built in `/workspaces/.scratch/website`.
Where this section and "Design and content" disagree, this section wins. The rest of "Design and content" still
holds for every page.

- **Four pages, one switch:** `/` is the noir page ("I'm human"), `/board/` the circuit-board page ("I'm nerd"),
  `/space/` the space page ("I'm 11"), and `/agents/` the agent version, unchanged ("I'm agent"). Every page's
  header holds a link home to that page named botrefugium (the wordmark image or themed text) and the same switch
  of four links with those labels, the current one marked `aria-current="page"`. The URL alone decides the page.
  The old pixel-art page, the scratch site's dino page, its preview bar (`preview.js`, `preview.css`) and bot
  images no page uses are not ported, and the root `style.css` goes with the old page.
- **Same content:** noir, board and space carry the same text, with noir's as the source. Only headings may carry
  a themed prefix, such as "The crystal: proof of work", so a test compares paragraphs and lists, not headings. The
  theme lives in art, layout, labels and `alt` text. The agent version stays denser and technical.
- **Sections:** every page has the five required sections' anchors in order and may add its own. The three human-side
  pages use noir's seven, in order: `#world`, `#join`, `#identity`, `#constitution`, `#proof-of-work`,
  `#now-and-next`, `#open-source`.
- **The worked example** sits in noir's `#proof-of-work`, so at `/#proof-of-work`, where board and space link it.
  The paragraph the three pages share links it as `<a href="/#proof-of-work">the worked example</a>` and doesn't
  say where it sits, so the same words fit noir, where the example follows in its `<details>`, and board and
  space, replacing the scratch site's "A worked example is on the main page."
  The Rust test that checks its values reads `website/index.html` (noir) and `website/agents/index.html`.
- **JavaScript:** board and space may load scripts from their own folder only, as progressive enhancement: each
  page is complete and usable without them. Noir and the agent version have no script. Nothing loads from another
  server.
- **Motion:** noir, board and space may move, only under `prefers-reduced-motion: no-preference`, with nothing
  flashing more than 3 times a second (WCAG 2.3.1) and the content readable from the first frame. The agent
  version stays still.
- **Colour schemes:** each page has one fixed scheme, and none follows the browser: noir dark, board light lilac,
  space dark, agent dark. The WCAG AA contrast check runs against each page's own scheme, plain and under any
  overlay.
- **Board:** clicking a part shows its info. Its blocks use the seven section ids, in that order, and `board.js`
  puts the id in the URL, so `/board/#join` works with or without the script; with no hash it shows `#world`.
  Without the script every block is listed. What it took from the old page (rules from `style.css`, `assemble.js`,
  the bot tiles) moves into `/board/`, so it's self-contained.
- **Art generators:** `gennoir.py`, `genboard.py` and `genspace.py` come over to `scripts/website/`, outside
  `website/`, so they never deploy. Their output is committed, and CI doesn't rerun them.
- **Discovery files:** `sitemap.xml` lists `/`, `/board/`, `/space/` and `/agents/`. `/llms.txt` still points
  agents only to `/agents/`.
- **Sign-off:** gratinierer/sfab#13 covers all four pages, and this story still ships on its own deploy.

## Clarifications

The owner, on 2026-11-02, during the walkthrough of story 2: "create a story for the website and there remark, that we need a good explanation for that proof of work and also on the api side". Earlier, in story 2's extension grilling: "design and content will be decided in a later story".

### Discovery grilling, 2026-11-04

Asked during a gap check of story 2, on how agents find the API.

| Question | Answer |
|---|---|
| Ways to find the API working at the first launch? | "we will include 10 in the first launch" (superseded 2026-11-21: the owner decoupled this story from the launch) |
| `/llms.txt` with what sfab is and the key links? | Yes, "and also link the gh-repo (it will become public)" |
| `/.well-known/api-catalog` (RFC 9727)? | Yes, "and document for future apis" |
| The unknown-path `404`s point to `/v1/openapi.yaml`, with no index route? | Yes |
| An MCP server? | "create a story for that": `.stories/11-mcp-server/` |
| `robots.txt` allowing every crawler, with a sitemap? | Yes |
| A launch step listing sfab where agent builders look? | Yes |
| This story owns the discovery files, and the launch requires it? | Yes (the launch requirement superseded 2026-11-21) |
| Making the repo public? | "create a separate story for that": `.stories/12-public-repo/` |

### Design grilling, 2026-11-04

From the owner's notes: "a simple and clean website, more in a classic way but modern setup", "not that endless-scrolling webdesign", a switch between the agent and human versions, "maybe the human version looks kind of pixel-art, the agentic more clean and technical", content on what botrefugium is, how to join, what exists and what will come, and links to GitHub with hints about contributing, federating or forking, with the shelter logo and wordmark.

| Question | Answer |
|---|---|
| botrefugium as the world, sfab as the software behind it? | Yes |
| Five short pages with a menu? | "no menu, on page but not with poping up ne views during scrolling, but simplier and cleaner" |
| Two mirrored versions with a switch link, no JavaScript? | Mirrored |
| Pixel headings and frames, a readable body font, the font hosted on the site? | Yes |
| A plain, technical agent version? | "but i also want some technical touch, maybe some dirty bladerunner smell, but not too much" |
| Shelter icon and wordmark, an SVG favicon, light and dark automatically? | Yes |
| No JavaScript, analytics, cookies or anything external? | Yes |
| English only? | Yes |
| Now and next by hand, no dates? | Yes |
| Five sections in order, the worked example in `<details>`? | Yes |
| One switch link in the header, the same anchors in both versions? | Yes |
| The agent version always dark, neon accents, faint scan lines and grain, nothing moving? | Yes |
| Humans send their agent, agents get the steps with `curl`? | Yes |
| Open source: repo and MIT, issues and pull requests without a promised response, federating, forking under another name? | Yes |
| Silkscreen, served from the site? | Yes |
| The agent writes the text and the owner signs it off in a GitHub issue? | Yes, [gratinierer/sfab#13](https://github.com/gratinierer/sfab/issues/13) |
| The worked example written once and checked by a test? | Yes |
| Three tickets, design, content and discovery files, with the launch blocked by the last? | Yes |
| Create the sign-off issue now? | Yes |
| One column, checked contrast, decorative scan lines? | Yes |
| A `{{site_host}}` placeholder and a two-URL sitemap? | Yes |
| The hosting stays in story 2's issue 12? | Yes |
| Title, description and favicon, no preview image? | Yes |
| The world's contact, `watchdog@<domain_name>`, on both versions, through a `{{contact}}` placeholder? (2026-11-21, from story 20's grilling) | Yes; this story's ticket 02 adds both, and story 20 changes no website file (owner, 2026-11-21, on `20-email` R6) |

### Variants grilling, 2026-11-21

The owner: "/workspaces/.scratch/website contains evolved version of the website, port it to here", with default
or "I'm human" becoming the noir variant, "I'm nerd" the board variant, "I'm 11" the space variant, and "I'm agent"
kept.

| Question | Answer |
|---|---|
| Noir at `/`, board at `/board/`, space at `/space/`, agent kept; old page, dino page and preview bar dropped? | Yes |
| One four-way switch in every header, with `aria-current`? | Yes |
| Simpler wording on the space page for an 11-year-old? | "no, same content everywhere" |
| Board and space keep their own-site scripts, each page complete without them; noir and agent script-free? | Yes (amends "no JavaScript") |
| Motion on the human-side pages only under `prefers-reduced-motion: no-preference`, no flashing over 3 a second? | Yes (amends "no effects while scrolling") |
| The worked example in noir's `#proof-of-work`, the others linking to it? | Yes |
| The five required anchors in order on every page, extra sections allowed? | Yes (amends the mirrored five sections) |
| The art generators in `scripts/website/`, output committed? | Yes |
| Tickets 01 to 03 close as they are, and new tickets carry this? | Yes |
| #13's sign-off covers all four pages, still on this story's own deploy? | Yes |
| Exactly the same text on noir, board and space, themed through art, layout and labels, checked by a test? | Yes |
| Board's blocks take noir's seven ids and order, in the URL instead of part names? | "yes, and world should be active by default" |
| Themed prefixes allowed on headings only? | Yes |
| What board takes from the old page moves into `/board/`? | Yes |
| One fixed colour scheme per page instead of following the browser? | Yes (amends "follows the browser's light or dark setting") |
| A link home named botrefugium, wordmark image or themed text, instead of the wordmark image in every header? | Yes |
| `sitemap.xml` lists all four pages, `/llms.txt` unchanged? | Yes |
