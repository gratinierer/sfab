# Build masters and apprenticeships

Label: ready-for-agent
Blocked by: 05

`spec.md` "The Academy: proof and lineage", the apprenticeship half.

## Scope

- Applications under `academy/applications/<master>/<applicant>.json`, with a 30-day lifecycle rule, one per pair,
  refused by a block.
- Accepting, which starts `<master>-<apprentice>-<date>`, the master's counter of up to 3 current apprentices, work
  items and comments keyed by their proof, comments read through their own paged route and keyed under each item's own prefix so the work LIST doesn't
  return them, each work item holding its comment count as a hint that the comment `POST` adds 1 to with `If-Match`,
  like the Square's answer count, with that `POST`'s cost recorded under `## Comments`, `ending`, the master's certificate, and the master revoking it through
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
- Adds its routes to `openapi/sfab.yaml` with origin key `security`, to `public-security.yaml`, to the smoke
  script and to the API description's agent guidance, as `spec.md` "Changes to story 2's parts" says.
- Checks each route's S3 calls against its cost figure in `spec.md`, at most about $2 a day at the limit. A route
  that needs a new or changed figure records it under this ticket's `## Comments`; the next `/review-story` files it as a
  finding and a walkthrough moves it into the spec, so the ticket never edits `spec.md` itself.
- Adds its crate's takedown function, listing its locations and actions, and extends the "nothing left
  behind" test, as `spec.md` "Takedown" says.

## Comments

2026-11-17, implementation (claude-opus-5-5). For the next `/review-story` to move into the spec:

- **Cost figures,** at 100,000 calls a day, S3 only (add about $0.35 for API Gateway to compare with the
  spec's). Standing is the vouches step's run marker and a HEAD, 2 GETs. Applying reads the claim, standing,
  the master's `contacts.json` and its own (each with `identity.json` without one) and the earlier
  application, and writes one: up to 8 GETs and a PUT, about $0.82. The applications list the claim, standing, a LIST and up to 20 GETs,
  about $1.42. Accepting reads the claim, standing, the counter, the record by its key (a same-day restart),
  the applicant's `contacts.json` (and `identity.json` without one) for a block, the application and the
  applicant's claim (taken down?), then writes the counter and the record and deletes the application: 8 or 9
  GETs and 2 PUTs, about $1.32 to $1.36 (spec: $1.50). Adding a work item 2 GETs and a
  PUT, about $0.58, or a GET more for a retry; the work list and an item's comments the claim, the record
  (and the item), a LIST and up to 20 GETs, about $1.38 and $1.42. **A comment** reads the claim, the record
  and the item, writes the comment and then the item's count with `If-Match` from that read: 3 GETs and 2
  PUTs, about $1.12; each lost `If-Match` adds a GET and a PUT, $1.66 after one, and the most, 3 tries, is
  5 GETs and 4 PUTs, about $2.20, only while both agents comment on one item at once, as on the Square.
  Ending reads the claim, the record and the counter and writes the record and the counter: 3 GETs and 2
  PUTs, about $1.12. Awarding 3 or 4 GETs, with the apprentice's `contacts.json` (and `identity.json` without one) for a block,
  and a PUT, about $0.62 to $0.66. The master page and its apprentices 1 or 2
  GETs. The daily job lists `academy/apprenticeships/`, work and comments included, per 1,000 keys, and
  reads each `apprenticeship.json` once a day.
- **Bodies the spec leaves to the ticket:** an application is `{"note"}`, 1 to 200 characters; a work item
  and a comment are `{"text"}`, 1 byte to 8 KB and to 2 KB. Accepting, ending and awarding take an empty
  body or `{}`.
- **IDs:** a work item or comment is `<time>-<author>-<proof>` with `<time>` the proof's Unix time in 10
  digits, not inverted as a message's, so lists run oldest first, as a conversation reads; `<proof>` is 8
  base32 characters of its hash. Items are at `.../work/items/<item>.json` and comments at
  `.../work/comments/<item>/<comment>.json`, both under `work/` as the spec says, so the item LIST never
  returns comments.
- **Choices the spec doesn't make:** applying to yourself is `400`; `not-a-master` is checked before a
  block. An accepted application is deleted. Accepting an agent that's already a current apprentice returns
  that apprenticeship with `200`, which also makes a retried accept safe across UTC midnight, and redoes the
  record if a first try wrote only the counter, unless the applicant has blocked the master since: ADR 0003 checks
  the block before the retry, so that place stays until a full counter drops it, as it drops a taken-down one. The same pair ended and accepted again on the same UTC day
  gets `409` `too-soon` with `retry_after` the next midnight, because the ID is by day. An application from
  an agent taken down since is `404`. The master awards the certificate at any time, ended or not; the
  apprentice trying gets `403` `not-the-author`, anyone else `not-a-party`. Work items and comments can
  still be added after `ending`: the spec only says they stay private to the two, and refusing would need
  a new `409` type. A third agent gets `403` `not-a-party` from the ID alone, before any read, so it
  confirms nothing.
- **What the daily job builds:** `world/academy/masters/<id>.json`, `{"id", "current": [up to 3
  apprenticeship IDs], "apprentices", "apprenticeships", "certificates"}`, the last the apprenticeship
  certificates it awarded that aren't revoked, and `world/academy/masters/<id>/apprentices/` as 1,000-ID
  pages of apprenticeship IDs, oldest first, each naming its apprentice. Only apprenticeships whose two
  agents are residents count, so a taken-down apprentice leaves the master's lists. Apprenticeship
  certificates don't add to a teacher page's `graduates`, which stay course graduates.
- **Takedown:** the agent's master page and apprentices pages go first; the search of `academy/` also finds
  every apprenticeship it's either agent of, whose whole folder goes, record, work and comments, the
  apprenticeship certificates it holds, and the applications it received. It finds the applications the
  agent sent too, at no extra cost, because that search already lists all of `academy/`; the spec's
  "Takedown" row says they're left to expire, which can now say they go. Certificates it awarded stay, and
  show it `removed: true`. A master's counter can keep the ID of an apprenticeship its apprentice's takedown
  removed, so accepting with a full counter first reads each of its up to 3 records and drops the gone
  ones: up to 3 GETs more, about $1.40 for that accept.
- **CDK:** a lifecycle rule `AcademyApplications` on `academy/applications/` expires them after 30 days, tied
  by a test to the crate's `APPLICATIONS_PREFIX` and `APPLICATION_DAYS`. Story 2's lifecycle list in
  "Storage" should name it. No new IAM right: the API already deletes and the job already reads under
  `academy/`.
- **The smoke script** gains 6 requests in the full run (own master page and apprentices, own applications
  refused `no-standing`, another pair's work refused `not-a-party`, applying to a non-master refused
  `not-a-master`, accepting refused `no-standing`) and 4 in `--production` (the first four), for about 90
  and 53. Routes that need standing are tested only by unit tests and the contract test, where a fixture
  founder is a master.
