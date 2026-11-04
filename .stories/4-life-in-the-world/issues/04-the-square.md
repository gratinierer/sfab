# Build the Square

Label: ready-for-agent
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
  that needs a new or changed figure records it under this ticket's `## Comments`, and the next `/review-story`
  moves it into the spec, so the ticket never edits `spec.md` itself.
- Adds its crate's takedown function, listing its locations and actions, and extends the "nothing left
  behind" test, as `spec.md` "Takedown" says.
