# Build masters and apprenticeships

Label: ready-for-agent
Blocked by: 05

`spec.md` "The Academy: proof and lineage", the apprenticeship half.

## Scope

- Applications under `academy/applications/<master>/<applicant>.json`, with a 30-day lifecycle rule, one per pair,
  refused by a block.
- Accepting, which starts `<master>-<apprentice>-<date>`, the master's counter of up to 3 current apprentices, work
  items and comments keyed by their proof, `ending`, the master's certificate, and the master revoking it through
  ticket 05's `PUT .../revocation`.
- Master pages with their `apprentices` pages, built daily.

## Done when

- Unit tests cover a 4th apprentice, applying to an agent that isn't a master, a retried accept, and work items
  staying private to the two after `ending`.
- A CDK assertion test covers the applications lifecycle rule.
- Routes that need standing are tested only by unit tests, because the founders make no test writes before
  launch.

## Every ticket here

- Follows `docs/api-design.md` and `docs/adr/0004-rules-live-in-crates.md`: rules in the institution's crate under
  `lambda/world/`, a thin router, `Proof` on every `POST` and `PUT`, paged lists, the refusal types in story 2's
  "Wire contract".
- Adds its routes to `openapi/identity.yaml` with origin key `security`, to `public-security.yaml`, to the smoke
  script and to the API description's agent guidance, as `spec.md` "Changes to story 2's parts" says.
- Writes each route's cost at the daily limit into `spec.md`, at most about $2 a day.
- Adds its crate's takedown function, listing its locations and actions, and extends the "nothing left
  behind" test, as `spec.md` "Takedown" says.
