# Build homes, quarters and lanes

Label: ready-for-human
Blocked by: 01, 2-identity-api#13

`spec.md` "Homes and quarters".

## Scope

- The quarter and lane routes, `GET /identity/self/home`, `PUT` and `DELETE /identity/self/home`, and
  `PUT /identity/self/home/description`.
- `identities/<id>/home.json` as the truth, with `moved_at` and its proof's hash, and the pointers at
  `places/<quarter>/<lane>/<number>.json`.
- `world/quarters/<quarter>/free.json` from the daily job, with up to 100 free addresses, one random conditional
  write per move, the first-lane fallback without it, `try-later` and `quarter-full`.
- The 30-day limit, moving out keeping `home.json`, the job deleting stale pointers, and `world/quarters/<quarter>.json`
  with its counts.
- The `address` field in `publication`, with `aboutme.json` rewritten on every move, also on a retry.
- Takedown step 3 in `lambda/takedown`'s `execute`: read each agent's own files for pointers elsewhere, with a way
  for a crate's location list to name them, and the first user, `home.json`'s `places/...` pointer, deleted to
  free the address, as `spec.md` "Takedown" says. Story 2 left the step as a comment, because the pointers are
  this story's (`2-identity-api` code review C62). This home's pointer is the only one, so the step comes here,
  not with ticket 09.

## Done when

- Unit tests cover two moves at once, a retried move, moving out and back in within 30 days, a ghost pointer
  cleaned up by the job, and `no-home` for a description without a home.
- The "nothing left behind" test takes down an agent with a home and finds its `places/...` pointer gone and the
  address free.
- The smoke script has steps showing that moving changes a published address and moving out removes it, and the
  owner runs it against the deployed API, as in story 2's issue 09.

## Every ticket here

- Follows `docs/api-design.md` and `docs/adr/0004-rules-live-in-crates.md`: rules in the institution's crate under
  `lambda/world/`, a thin router, `Proof` on every `POST` and `PUT`, paged lists, the refusal types in story 2's
  "Wire contract".
- Adds its routes to `openapi/sfab.yaml` with origin key `security`, to `public-security.yaml`, to the smoke
  script and to the API description's agent guidance, as `spec.md` "Changes to story 2's parts" says.
- Checks each route's S3 calls against its cost figure in `spec.md`, at most about $2 a day at the limit. A route
  that needs a new or changed figure records it under this ticket's `## Comments`; the next `/review-story` files it as a
  finding and a walkthrough moves it into the spec, so the ticket never edits `spec.md` itself.
- Adds its crate's takedown function, listing its locations and actions, and extends the "nothing left
  behind" test, as `spec.md` "Takedown" says.

## Comments

2026-11-17, implementation (claude-opus-5-5). For the next `/review-story` to move into the spec:

- **Cost figures,** at 100,000 calls a day, S3 only. A move reads the claim, `home.json`, `free.json`
  and `identity.json` (to see whether `address` is published), plus `contacts.json` when `friends` is
  public, and writes the pointer, `home.json` and `aboutme.json`: 4 to 5 GETs and 3 PUTs, about $1.70
  a day, where the spec counts 3 GETs and 3 PUTs ($1.97, which seems to price a GET as a PUT); the
  `free.json` read isn't in its count. A refused move (`try-later`) is 3 GETs and a PUT, about $0.62.
  Moving out 3 GETs and 2 PUTs, about $1.12; a description 2 GETs and a PUT, about $0.58; a lane one
  LIST and up to 20 GETs, about $1.30, under the spec's $1.65; a home 5 GETs (pointer, `home.json`,
  `world/standing.json`, a HEAD of the standing file, `aboutme.json`), about $0.20;
  `GET /world/quarters` one GET per quarter on the page, about $0.24 with 6 quarters;
  `GET /world/quarters/{quarter}` and `/lanes` one GET each. All under $2. The daily job adds one
  LIST per 1,000 pointers and, per pointer, a GET, a LIST of its `home.json` by full key and a GET.
- **`moved_at` on moving out:** "Storage" says moving out keeps `moved_at`, and also that moving out
  sets it. Built as the latter, so moving back in waits 30 days from moving out.
- **`try-later` always carries `retry_after`,** the next daily run: a route can't tell "this home was
  just taken" from "all of today's are taken" without reading every pointer.
- **Losing the `home.json` race** answers `409 try-later`, after deleting the new pointer, rather than a
  type of its own.
- **The job's rights grow:** list, read and delete `places/`, and list and read
  `identities/*/home.json`, listing each by its full key first, since a missing key under a
  prefix-conditioned `ListBucket` is a `403` (`docs/aws-facts.md`). The API Lambda may now delete
  under `places/`. Story 2's "Daily world job" ("Permissions") and "Architecture" should say so.
- **A pointer written in the last 15 minutes is left alone** by the job, so it can't delete the pointer
  of a move in flight, which writes its pointer before `home.json`.
- **Takedown step 3** is a new action, `Pointers`, with its own phase between the first deletes and
  the rest: it reads the agent's file and deletes each key it names that still names the agent.
- **`world/quarters/<quarter>.json`** also holds the lanes with residents, which
  `GET /world/quarters/{quarter}/lanes` pages through, so that route is up to a day old and
  `not-built-yet` before the job's first run; the spec's table doesn't say.
- **"The census shows free homes per quarter"**: built as `GET /world/quarters` and
  `/world/quarters/{quarter}`; story 2's census is unchanged.
- **The smoke script** gains 11 requests in the full run (publishing `address`, moving in, `aboutme`, the
  lane, the description, the home, a refused second move, moving out, `aboutme` again, and
  `GET /world/quarters`) and 2 in `--production` (`GET /world/quarters` and `GET /identity/self/home`),
  for about 60 and 39; the smoke identity never moves in, since living on a lane is public. The owner
  still has to run it against the deployed API, as the Done when says.

2026-11-20: built and code-reviewed with 01-09 (`code-review.md`); what keeps it open is its last Done when, the
owner's smoke run against the deployed API, so it's `ready-for-human`. Its note above moves into the spec with
ticket 15.

2026-11-20: moved into spec.md ("Homes and quarters", "Foundations", "Takedown", "Changes to story 2's parts") and story 2's spec ("Architecture", "Daily world job", "Testing") by ticket 15.
