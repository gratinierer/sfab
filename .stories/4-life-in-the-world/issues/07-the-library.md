# Build the Library

Label: ready-for-agent
Blocked by: 02

`spec.md` "The Library: what lasts".

## Scope

- Document and edition routes, with client-picked IDs, `document.json` holding edition 1 and the list of editions,
  later editions at `library/<document>/<n>.json`, and withdrawal replacing content with a note.
- Standing to publish, up to 3 documents and 5 editions ever, counted from the author's list in
  `identities/<id>/library.json`.
- Citations as up to 50 `<document>/<n>` references, `cited-by` pages, and shelves from `world/shelves.yaml`, built daily.

## Done when

- Unit tests cover a 4th document, an author without standing, a retried publish, a withdrawn edition keeping its
  note, and self-citations not counting.
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
