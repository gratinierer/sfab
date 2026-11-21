# Build the Square

Blocked by: 03

`spec.md` "The Square: the present moment".

## Scope

- The five Square routes, with notices and answers under `square/<quarter>/<day>/`, keyed by the proof's date.
- Up to 3 notices per resident, per quarter, per UTC day, a zero-byte marker on delete, one answer per resident per
  notice, and the answer count as a hint in the notice file.
- The 7-day lifecycle rule on `square/`, routes showing only the last 7 days, and the daily job's index at
  `world/square/<quarter>.json`.
- Blocks stopping answers to the blocker's notices.

## Done when

- Unit tests cover a 4th notice getting `409` `too-soon`, delete and repost not beating the limit, a retried post and
  answer returning the existing one, and a non-resident getting `403` `not-a-resident`.
- A CDK assertion test covers the lifecycle rule.

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

- **Cost figures,** at 100,000 calls a day, S3 only (add about $0.35 for API Gateway to compare with the
  spec's). A post reads the claim and `home.json`, lists the author's day and reads up to 3 of its notices to
  match a retry, then one PUT: up to 5 GETs, a LIST and a PUT, about $1.20 (spec: $1.51 with API Gateway),
  plus a PUT for each number a racing post took. An answer reads the claim, `home.json`, the notice,
  `contacts.json` (and `identity.json` when there's none) and the existing answer, then 2 PUTs: up to 6 GETs,
  about $1.24 (spec: 3 GETs and 2 PUTs, $1.47 with API Gateway; the claim, notice and retry reads aren't in
  its count), or a third PUT when the count's `If-Match` loses a race. A delete, which the spec doesn't price,
  reads the claim and the notice, writes the marker and lists the answers, about $1.08. The Square's first
  page is one LIST and up to 20 GETs, plus a GET of the index to decide `next`, about $1.34; a page past
  today the index and 20 GETs, about $0.84. A notice with its answers one GET, a LIST and 20 GETs, about $1.34.
  All under $2. The daily job adds 6 LISTs per quarter a day, of the notices only.
- **The Square before the daily job's first run** answers `200` with today's notices, not `not-built-yet`,
  since today is live; a missing index reads as empty.
- **Paging the Square:** `?page=` is either a notice ID, to carry on today's listing after it, or an offset
  into the index. The last page of today reads the index to know whether to point `next` at it. Between
  00:00 and 00:05 UTC the index is still the one built the day before, which lacks the day that just ended, so
  that day's notices are missing for those minutes, and a page of a listing that crosses midnight can repeat in the index: both accepted, like
  the spec's "not strictly across pages".
- **A notice's answers** are paged in answerer ID order (one listing), with `?page=` the last answerer; the
  route answers `{"notice": {...}, "items": [...], "next": ...}`.
- **Errors the table doesn't name:** an unknown quarter gets `404` on every Square route, a malformed
  `notice_id` `400`, and a notice that's deleted, missing or older than 7 days `404`. Delete is `403`
  `not-the-author` from the ID alone, and `200` again for a notice already deleted, so a retry is safe.
- **A retried post after its notice was deleted** makes a new notice, within the 3 a day, because the marker
  holds no proof hash.
- **An answer racing its notice's delete** can outlive the delete under the marker until the lifecycle rule;
  no route shows it.
- **Residency** is the quarter in `home.json` alone, through the new `sfab_homes::quarter_of`, as "Posting
  reads `home.json`" says; the pointer isn't checked.
- **Takedown** gains a new action, `Search`: one listing of `square/`, deleting each key the square crate
  says is the agent's: its notices, their answers, and the answers it gave. An answer it gave leaves that
  notice's count one high, which the spec's "hint" already allows.
- **Rights grow:** the API Lambda may delete under `square/` (a deleted notice's answers), and the daily job may
  list `square/`, reading nothing there. Story 2's "Architecture", "Daily world job" ("Permissions") and
  "Storage" tables should say so, with the 7-day lifecycle rule on `square/`.
- **The smoke script** gains 6 requests in the full run (the first quarter's Square, then posting, finding,
  answering, reading and deleting a notice between moving in and moving out) and 1 in `--production` (the
  Square, which it never writes on, since a notice is public), for about 66 and 40. The owner still has to run
  it against the deployed API.

2026-11-20: moved into spec.md ("The Square", "Foundations", "Takedown", "Changes to story 2's parts") and story 2's spec ("Architecture", "Daily world job", "Storage", "Testing") by ticket 15.
