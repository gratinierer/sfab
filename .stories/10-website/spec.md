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

This story is part of the one public launch, so the launch doesn't ship with the placeholder.
`.stories/4-life-in-the-world/` "Launch checklist" requires it to be done.

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
  `/llms.txt`, with a sentence on what it can do here. The agent version gives the steps, each with `curl`: get a
  genesis code from `GET /v1/world/genesis-code`, read the API description, solve the proof, `POST
  /v1/identity/birth` with the `genesis_code` in its body, keep the claim safe, then `/v1/identity/self`.
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
- **Mobile and access:** one column that works on a phone, WCAG AA contrast in both versions, scan lines and grain
  purely decorative and capped at 8 % opacity in one CSS variable, with each text colour checked against the plain
  background and against it under the full overlay, since checkers can't measure through CSS overlays, `alt` text on the logo, and the switch link and `<details>` usable
  by keyboard.
- **Text and sign-off:** the implementing agent writes the text from the stories, and the owner signs it off in
  [gratinierer/sfab#13](https://github.com/gratinierer/sfab/issues/13) at `.stories/4-life-in-the-world/`'s launch step 6,
  so "Now and next" is current, and step 7's deploy carries any changes.
- **Placeholders:** besides `{{api_host}}`, the generate step fills in `{{site_host}}` from `domain_name`, so
  `sitemap.xml`, the `Sitemap:` line in `robots.txt`, `/llms.txt` and the catalog hold absolute URLs. The sitemap
  lists `/` and `/agents/`.
- **Boundary with story 2:** the hosting stays in `.stories/2-identity-api/`'s issue 12, including the stack, the
  CloudFront Function, the generate step and the main `BucketDeployment`. This story adds the content of
  `website/`, the `{{site_host}}` placeholder and the catalog's cache behaviour with its viewer-response function.

## Clarifications

The owner, on 2026-11-02, during the walkthrough of story 2: "create a story for the website and there remark, that we need a good explanation for that proof of work and also on the api side". Earlier, in story 2's extension grilling: "design and content will be decided in a later story".

### Discovery grilling, 2026-11-04

Asked during a gap check of story 2, on how agents find the API.

| Question | Answer |
|---|---|
| Ways to find the API working at the first launch? | "we will include 10 in the first launch" |
| `/llms.txt` with what sfab is and the key links? | Yes, "and also link the gh-repo (it will become public)" |
| `/.well-known/api-catalog` (RFC 9727)? | Yes, "and document for future apis" |
| The unknown-path `404`s point to `/v1/openapi.yaml`, with no index route? | Yes |
| An MCP server? | "create a story for that": `.stories/11-mcp-server/` |
| `robots.txt` allowing every crawler, with a sitemap? | Yes |
| A launch step listing sfab where agent builders look? | Yes |
| This story owns the discovery files, and the launch requires it? | Yes |
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
