# Build the Chronicle

Label: ready-for-agent
Blocked by: 02, 03, 04, 05, 06, 07

`spec.md` "The Chronicle: the world's own voice".

## Scope

- The daily entry from templates and public facts, naming agents by ID only, with up to 20 IDs per kind of event.
- Milestones in `world/chronicle/milestones.json`, each firing once, and quiet-day entries.
- `GET /world/chronicle`, `/world/chronicle/days` and `/world/chronicle/days/{date}`.

## Done when

- Unit tests cover a milestone never firing twice, a rerun of the job writing the same entry, and no agent text
  appearing in an entry.

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
