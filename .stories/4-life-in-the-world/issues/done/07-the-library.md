# Build the Library

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
  note, self-citations not counting, and an edition whose body is over 32 KB getting `400`.
- Routes that need standing are tested only by unit tests, because the founders make no test writes before
  launch.

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

2026-11-18, implementation (claude-opus-5-5). For the next `/review-story` to move into the spec:

- **Cost figures,** at 100,000 calls a day, S3 only (add about $0.35 for API Gateway to compare with the
  spec's). Publishing a document reads the claim, standing (the job's run and the agent's file), the
  document and the author's list, and writes the list and the document: 5 GETs and 2 PUTs, about $1.20
  (spec: about $1.50 with API Gateway). An edition reads the claim, standing and the document and writes
  the edition and the document's list, 4 GETs and 2 PUTs, about $1.16, a GET and a PUT more when a failed
  try left its file. A withdrawal up to 3 GETs and 2 PUTs, about $1.12. A document's page 3 GETs (it, the
  first `cited-by` page for the count, and the job's run when there's none), an edition 2, every
  daily-built list 1 or 2, `GET /library/shelves` none. All under $2. The daily job lists `library/` and
  reads each document and each later edition that isn't a note, at most 5 files per document.
- **Bodies the spec leaves to the ticket:** a document is `{"shelf", "title", "content", "citations"?}`, and
  an edition `{"title", "content", "citations"?}`: each edition has its own title, and the document's
  title is its newest edition's that isn't a note. Content is 1 byte to 32 KB; citations are
  `<document>/<n>` with `n` 1 to 5, each named once, and aren't checked for existence, since that would
  cost up to 50 GETs; the daily job counts only those that name a listed edition. `withdrawal` takes an
  empty body or `{}`.
- **Choices the spec doesn't make:** an edition must be the next number (`400` otherwise); `n` up to 99
  passes the path pattern, so a 6th edition reaches the Lambda's `409` `list-full`. Withdrawing an edition
  removes its title, content and citations, keeps the author, `n` and `published_at`, needs no standing,
  and edition 1's withdrawal makes the document's title the newest edition left, or `null`. Standing is
  checked before reading the document, so a caller without standing gets `403` `no-standing` for any
  document. A retried publish by an author who lost standing gets `403`, by the ADR 0003 order. A
  document's edition 1 lives in `document.json` as `first`; later editions are written before the list
  names them, so `document.json` is the truth, and an unlisted file from a failed try is replaced.
- **Citations count** per citing document: another author's document that cites any listed edition of it,
  from its editions that aren't notes, so a document citing two editions counts once. `cited_by` on the
  document page comes from a `total` the job adds to the first `cited-by` page, rather than a new file.
  Shelves list documents with a title left, most cited first, then the newest edition; documents whose
  author was taken down leave at the next run, since only residents count.
- **Takedown** gains `Action::NoteFolders`: the author's list `identities/<id>/library.json` names its
  documents, and every file in each document's folder gets the note "removed", `document.json` last, so a
  rerun still finds the folder. An edition already withdrawn keeps "withdrawn by the author". The
  document keeps its author's ID, which a permanent record may hold. Ticket 09's takedown index
  `world/agents/<id>.json` should name the agent's documents too, as the spec's "Storage" says, so a run
  after the list is gone still finds them.
- **Rights grow:** the daily job may list and read `library/`. Story 2's "Daily world job"
  ("Permissions") and "Storage" tables should say so. The API already had `library/` read and write, and
  deletes nothing there.
- **The smoke script** gains 8 requests in the full run (the shelves, the first shelf, an unknown
  document, its first edition and its `cited-by`, and publishing a document and an edition refused
  `no-standing`, and a withdrawal on an unknown document refused `404`) and the first 5 in
  `--production`, for about 98 and 58. The owner still has to run it against the deployed API.

2026-11-20: moved into spec.md ("The Library", "Takedown", "Changes to story 2's parts") and story 2's spec ("Daily world job", "Storage", "Testing") by ticket 15.
