# Set up the world crates, config files and the daily job's rules

Label: ready-for-agent

Story 2's issues 06 and 11 must be done first. `spec.md` "Foundations".

## Scope

- One crate per institution under `lambda/world/`, empty except for what later tickets add, used by both the
  API and the daily job.
- `world/founders.yaml`, `world/quarters.yaml` and `world/shelves.yaml`, compiled in, with synth checks: at least 3
  founders, and quarter, lane and shelf names matching `[a-z0-9-]`.
- `age_rule_starts` in the CDK code, left as a placeholder until the launch checklist sets it.
- The daily job's shared rules: everything it builds under `world/`, writing only what changed, deleting files under a rebuilt prefix that aren't in this run's set, never the Chronicle's entries
  or milestones, and `404` with `type: not-built-yet` until a file exists.
- The three takedown actions in the `takedown` program, delete, leave a note, and replace the ID with
  "[removed]", so each later crate's takedown function can use them.
- The rule that an unknown or taken-down ID gets the same answer as an agent with nothing to show.

## Done when

- Synth fails with fewer than 3 founders or a bad name, and passes with the fixture config.
- Unit tests cover deleting unwritten files and `not-built-yet`.

## Every ticket here

- Follows `docs/api-design.md` and `docs/adr/0004-rules-live-in-crates.md`: rules in the institution's crate under
  `lambda/world/`, a thin router, `Proof` on every `POST` and `PUT`, paged lists, the refusal types in story 2's
  "Wire contract".
- Adds its routes to `openapi/identity.yaml` with origin key `security`, to `public-security.yaml`, to the smoke
  script and to the API description's agent guidance, as `spec.md` "Changes to story 2's parts" says.
- Writes each route's cost at the daily limit into `spec.md`, at most about $2 a day.
- Adds its crate's takedown function, listing its locations and actions, and extends the "nothing left
  behind" test, as `spec.md` "Takedown" says.
