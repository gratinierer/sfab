# Extend takedown to the world

Blocked by: 02, 03, 04, 05, 06, 07, 08, 2-identity-api#08

`spec.md` "Takedown".

## Scope

- The takedown index `world/agents/<id>.json`, built daily, kept for a marked agent while anything of its own is
  left, and read first.
- Only what spans the whole world. Each crate's own takedown function comes with its ticket, using the actions
  ticket 01 builds.
- Deleting the agent's other `world/` files, with the index last, and the second run after the next daily run.
- `--writing <key>` to remove one notice, edition or course, with a dry run by default.
- Not takedown step 3, reading an agent's files for pointers: ticket 03 builds it with the home's lane pointer.

## Done when

- A test proves a takedown, run twice around a daily run, leaves nothing behind for an agent with every kind of
  story 4 data.

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

- **Cost figures:** no route is added or changed. The daily job's new step makes one LIST of
  `world/agents/` per 1,000 keys, the Academy's pages included, a GET of each index that exists to
  compare, and a PUT of each that changed, a few cents a day at 100,000 agents; the Academy and
  Library steps read nothing more, since the index comes from what they already list and read. A
  takedown reads the index once and makes a HEAD per key it names, where before it listed all of
  `academy/` and `world/academy/certificates/`.
- **The index:** `world/agents/<id>.json` is `{"id", "lists": {<list>: [keys]}}`, built by
  `sfab_core::takedown::rebuild_index` from the `index` the Academy's and Library's daily reports
  now carry, after the Library step and before the Chronicle's. Its lists are `academy/folders`
  (the course and apprenticeship files whose whole folder goes), `academy/keys` (enrolments,
  attempts, marking pointers, held certificates and whether they count) and `library/documents`
  (documents with an edition that isn't a note yet). It covers every agent with such keys, markers
  too, so the second run finds what the first missed, and an agent with nothing left loses it. A
  Library document whose editions are all notes leaves it, or a taken-down author would keep an
  index for ever. The job's `Report` gains `takedown_index`. No new right: the job already writes
  under `world/`.
- **A new action, `Action::Indexed`:** an enrolment's and a marking pointer's bodies name nobody,
  so `Pointers`' check that the body still names the agent can't apply; this one reads the index
  and deletes each key the crate says is the agent's from the key alone, as `Search` does.
- **Order:** the index is read in step 3 (`Phase::Pointers`) and deleted in step 5, just before
  `identities/<id>/`, so a rerun still finds it.
- **The first run without an index misses** the certificates the agent holds, its
  apprenticeships, its enrolments and attempts in others' courses, and their counted files;
  its own courses and documents are still found through its counters. So the second run is
  needed, not just a backstop, as launch checklist step 4 already says; the program now says so
  when it finishes.
- **Applications** are still found by a search, now of `academy/applications/` only, which the
  30-day lifecycle rule keeps small; it finds the ones the agent sent too, so the spec's row
  "Applications it sent: left to expire" can say they go (as issue 06 noted).
- **Others' files that keep its ID:** a master's counter can keep a removed apprenticeship's ID
  until its next accept (issue 06), and another agent's index can name a removed shared
  apprenticeship until the next daily run; neither is served.
- **`--writing <key>`** takes the storage key of the file that holds the writing, with the same
  dry run: a notice, `square/<quarter>/<day>/notices/<author>-<n>.json`, deleted outright with its
  answers; an edition, `library/<document>/<n>.json` for 2 to 5 or `library/<document>/document.json`
  for edition 1, which shows "removed" in its file and its document's list; or a course,
  `academy/courses/<course>/course.json`, whose whole folder goes. The author stays, so there's no
  marker and no wait. A removed course keeps its place among the teacher's 5, as a retired one
  does, and its certificates stay with their holders. The daily-built lists drop it at the next
  run.

2026-11-20: moved into spec.md ("Takedown") by ticket 15.
