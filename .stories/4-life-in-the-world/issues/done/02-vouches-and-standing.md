# Build vouches and standing

Blocked by: 01, 2-identity-api#13

`spec.md` "Vouches: trust".

## Scope

- `PUT`, `DELETE /identity/self/vouches/{id}`, `GET /identity/self/vouches`, `GET /identity/self/vouched-by`,
  `GET /identity/{id}/vouched-by` and `GET /identity/{id}/standing`.
- `vouches/<id>.json` with up to 10 active and a reason of up to 200 characters, conditional writes, refusing a vouch
  for an agent that blocked the giver.
- The daily job working standing outward from the founders, reading only the vouch files of agents with standing,
  with 3 vouches and 30 days of age, the launch exception, and no grace period.
- `world/vouches/<id>.json` and `world/standing/<id>.json`, and the `vouches` field in `publication`, with
  `aboutme` linking to `/identity/{id}/vouched-by`.

## Done when

- Unit tests cover a ring of fake agents gaining nothing, the age rule and its exception, a cascade after a voucher
  loses standing, a blocked giver getting `403` `blocked`, and an 11th vouch getting `409` `list-full`.
- `standing` answers an unknown ID like any agent without it.

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

- **Cost figures,** which `spec.md` has none of for these routes, at 100,000 calls a day, S3 only:
  `PUT /identity/self/vouches/{id}` reads the claim, the recipient's `contacts.json` and the giver's
  `vouches/<id>.json` and writes it, 3 GETs and a PUT, about $0.62 a day; `DELETE` 2 GETs and a PUT,
  about $0.58; `GET /identity/self/vouches` 2 GETs, `GET /identity/self/vouched-by` 3 (claim, file, and
  `world/standing.json` when the file is missing), `GET /identity/{id}/vouched-by` 3 (`identity.json`,
  file, marker) and `GET /identity/{id}/standing` 2 (`world/standing.json` and a HEAD), each about
  $0.12 or less. All under $2. The daily job adds one LIST per 1,000 keys of `vouches/` and of `vitals/`,
  one GET per agent with standing and per candidate's `vitals/<id>.json`, and the two rebuilds.
- **`world/standing.json`,** `{"as_of": ...}`, written last each run, is a file the spec's list under
  "What the daily job writes" doesn't name. It's what tells "not built yet" from "nothing to show" for
  both `world/standing/` and `world/vouches/`, and gives `standing`'s `as_of`, so the per-agent files
  needn't change daily.
- **The daily job's rights grow:** list and read `vouches/` and `vitals/` (`born_at`, so it never opens
  a claim), and the `AGE_RULE_STARTS` environment variable. Story 2's "Daily world job" ("Permissions")
  should say so.
- **"Skipping anyone with a takedown marker"** is done by the daily job, which counts only residents, as
  `sfab_identity::residents` lists them: a taken-down giver can still show in a `vouched-by` page until
  the next run, rather than the routes reading a claim per item.
- **A taken-down founder has no standing,** though still in `founders.yaml`, so `standing` answers its ID
  like any other taken-down one.
- **The smoke script** gains 6 requests, for about 49 in the full run and 37 in `--production`; story 2's
  "Testing" says about 43 and 31.

2026-11-20: moved into spec.md ("Vouches", "Foundations") and story 2's spec ("Daily world job", "Testing") by ticket 15.
