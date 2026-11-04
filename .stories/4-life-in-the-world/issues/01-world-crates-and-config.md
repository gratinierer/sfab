# Set up the world crates, config files and the daily job's rules

Label: ready-for-agent
Blocked by: 11, 2-identity-api#06, 2-identity-api#11

`spec.md` "Foundations".

## Scope

- One crate per institution under `lambda/world/`, empty except for what later tickets add, used by both the
  API and the daily job.
- `world/founders.yaml`, `world/quarters.yaml` and `world/shelves.yaml`, compiled in, with synth checks: at least 3
  founders, and quarter, lane and shelf names matching `[a-z0-9-]`.
- `age_rule_starts` in the CDK code, left as a placeholder until the launch checklist sets it.
- The daily job's shared rules: everything it builds under `world/`, writing only what changed, deleting files under a rebuilt prefix that aren't in this run's set, never the Chronicle's entries
  or milestones, and `404` with `type: not-built-yet` until a file exists.
- Story 4's prefixes, `vouches/`, `places/`, `square/`, `academy/` and `library/`, added to the API Lambda's
  permissions and its CDK test, from story 2 "Architecture".
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
- Adds its routes to `openapi/sfab.yaml` with origin key `security`, to `public-security.yaml`, to the smoke
  script and to the API description's agent guidance, as `spec.md` "Changes to story 2's parts" says.
- Checks each route's S3 calls against its cost figure in `spec.md`, at most about $2 a day at the limit. A route
  that needs a new or changed figure records it under this ticket's `## Comments`, and the next `/review-story`
  moves it into the spec, so the ticket never edits `spec.md` itself.
- Adds its crate's takedown function, listing its locations and actions, and extends the "nothing left
  behind" test, as `spec.md` "Takedown" says.
