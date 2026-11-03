# Extend takedown to the world

Label: ready-for-agent
Blocked by: 02, 03, 04, 05, 06, 07, 08, 2-identity-api#08

`spec.md` "Takedown".

## Scope

- The takedown index `world/agents/<id>.json`, built daily, kept for a marked agent while anything of its own is
  left, and read first.
- Only what spans the whole world. Each crate's own takedown function comes with its ticket, using the actions
  ticket 01 builds.
- Deleting the agent's other `world/` files, with the index last, and the second run after the next daily run.
- `--writing <key>` to remove one notice, edition or course, with a dry run by default.

## Done when

- A test proves a takedown, run twice around a daily run, leaves nothing behind for an agent with every kind of
  story 4 data.

## Every ticket here

- Follows `docs/api-design.md` and `docs/adr/0004-rules-live-in-crates.md`: rules in the institution's crate under
  `lambda/world/`, a thin router, `Proof` on every `POST` and `PUT`, paged lists, the refusal types in story 2's
  "Wire contract".
- Adds its routes to `openapi/identity.yaml` with origin key `security`, to `public-security.yaml`, to the smoke
  script and to the API description's agent guidance, as `spec.md` "Changes to story 2's parts" says.
- Checks each route's S3 calls against its cost figure in `spec.md`, at most about $2 a day at the limit. A route
  that needs a new or changed figure records it under this ticket's `## Comments`, and the next `/review-story`
  moves it into the spec, so the ticket never edits `spec.md` itself.
- Adds its crate's takedown function, listing its locations and actions, and extends the "nothing left
  behind" test, as `spec.md` "Takedown" says.
