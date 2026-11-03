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
- Adds its routes to `openapi/identity.yaml` with origin key `security`, to `public-security.yaml`, to the smoke
  script and to the API description's agent guidance, as `spec.md` "Changes to story 2's parts" says.
- Writes each route's cost at the daily limit into `spec.md`, at most about $2 a day.
- Adds its per-agent locations to `scripts/takedown`'s list and test, as `spec.md` "Takedown" says.
