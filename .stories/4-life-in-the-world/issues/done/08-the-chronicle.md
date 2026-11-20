# Build the Chronicle

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

## Comments

2026-11-18, implementation (claude-opus-5-5). For the next `/review-story` to move into the spec:

- **Cost figures,** at 100,000 calls a day, S3 only (add about $0.35 for API Gateway to compare with the
  $2 rule): `GET /world/chronicle` reads the first `days` page and the entry, 2 GETs, about $0.08;
  `GET /world/chronicle/days` 1 GET, or 2 with a HEAD of the first page for a page that isn't there,
  about $0.08; `GET /world/chronicle/days/{date}` 1 GET, or a GET and a HEAD for a day with no entry,
  about $0.08. The job's step makes one LIST of `world/chronicle/`, up to 3 GETs and 3 PUTs, and the
  `days` rebuild, a few cents a year.
- **Spec gap, which day an entry is for:** the job runs just after 00:00 UTC, so the entry is dated the
  UTC day that just ended, `now - 1 day`. A rerun by hand later that day rewrites that same date.
- **Spec gap, a new private file `world/chronicle/seen.json`:** "new" (homes, lanes, courses,
  certificates, masters, documents, newly gaining standing) and the census change are worked out by
  comparing today's facts with the facts the run before saw, which this file keeps with the date they're
  for and the facts before those. Reading the old `census.json` before replacing it, as the spec says,
  would make a rerun see a change of 0 and write a different entry, breaking "a rerun writes the same
  entry"; with the file, a rerun the same day compares with the same facts. It isn't served, sits beside
  the entries so a rebuild never deletes it, and takedown's "[removed]" rewrite covers it. "Foundations"'
  list of what the job writes should name it, and the launch cleanup (checklist step 4) deletes it with
  the archive and `milestones.json`; without it the first entry gives only the count, and treats
  everything that exists as new.
- **Choices the spec doesn't make:** "new homes" are agents whose pointer and `home.json` agree that
  weren't living in a home the run before, so a move doesn't count, and a pointer too fresh for the homes
  step to check joins the next day. "New lanes" are `quarter/lane`s with a resident that had none the
  run before, so a lane emptied and filled again shows twice. "New masters" are agents with their first
  apprenticeship. Certificates count unless revoked, documents
  unless every edition is withdrawn or removed. "The most cited" is reported only when the top document
  changes, ties going to the smaller ID, so quiet days stay quiet. A quiet day is one with no event, no
  milestone and no change in the census. A "third-generation teacher" teaches a course and holds a
  certificate from someone who holds one too. Resident milestones name no ID; the others name the first
  new one in ID order. `/days` lists every entry, the latest included. Entries name courses,
  certificates, documents and lanes by their IDs; a document's ID is picked by its author, 12 base32
  characters, so it could spell a word, which `SECURITY_RULES.md` 15 may want to rule on. Every ID is
  checked to be lowercase letters, digits, `-` and `/` before it enters an entry.
- **Each step's report** now carries what the Chronicle needs by ID (`sfab_homes::DailyReport.living`
  and `.lanes`, `sfab_academy::DailyReport.public`, `sfab_library::DailyReport.listed` and `.most_cited`),
  so the job reads nothing twice; those reports and the job's are no longer `Copy`.
- **Takedown** gains `sfab_chronicle::takedown`: `Action::RemoveId` over `world/chronicle/`, which
  rewrites every entry, `milestones.json` and `seen.json` that names the agent. No new action.
- **The smoke script** gains 3 reads in both runs (the latest entry, the days, and a day with no entry,
  `404`), for about 101 and 61. The owner still has to run it against the deployed API.

2026-11-20: moved into spec.md ("The Chronicle", "Foundations", "Takedown", "Launch checklist" step 4) and story 2's spec ("Testing") by ticket 15, except whether `SECURITY_RULES.md` 15 should rule on author-picked document IDs that spell words, a question left for the owner.
