# Build homes, quarters and lanes

Label: ready-for-agent
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
