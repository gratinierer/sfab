# Build Academy courses, exams and certificates

Blocked by: 02

`spec.md` "The Academy: proof and lineage", the exam half.

## Scope

- Course routes: publish, edit, retire, the page without tasks, `tasks` for the teacher, and `enrolled` and
  `graduates` pages.
- Enrolment under `self`, attempts keyed `<student>-<date>`, per-task `answers/{n}` up to 8 KB stored in the attempt file with `If-Match`, `submission`, and
  marking by the teacher or a graduate, with the queue as pages of attempt IDs and `GET .../marking/{attempt}`
  for one attempt's answers.
- Exam certificates `exam-<course>-<holder>`, issued at once and counted daily from the marker's or teacher's standing,
  revocation by the teacher, and `GET /academy/certificates/{certificate}` for both kinds of
  certificate.
- The catalogue ranked by the teacher's standing, teacher pages with counts only, `/identity/{id}/enrolments` and
  `/certificates`, and the counter of a teacher's courses in `identities/<id>/academy.json`.

## Done when

- Unit tests cover a 6th course, a retake within 7 days, a non-enrolled attempt, enrolling in a retired course, a
  graduate who can't revoke, tasks never appearing on the public course page, and answer hashes never returned by
  `GET .../attempts/{attempt}`, two answers handed in at once both being kept, and an answer after submission
  getting `409` `submitted`.
- The task set per student is fixed by its ID, an edit during an open attempt doesn't change it, and a retried
  submission returns the first.
- A certificate whose teacher has a takedown marker shows "removed", and a taken-down agent's own
  certificates are deleted by this crate's takedown function.
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
  spec's). Publishing reads the claim, the course (a retry) and the counter, then writes the counter and the
  course: 3 GETs and 2 PUTs, about $1.12. Editing, retiring, enrolling and revoking 2 GETs and a PUT, about
  $0.58; leaving a GET and a free DELETE. Starting an attempt reads the claim, the course, the enrolment and
  the latest attempt, lists the student's attempts and writes one: 4 GETs, a LIST and a PUT, about $1.16. An
  answer 2 GETs and a PUT, about $0.58, plus a GET and a PUT per lost `If-Match` (spec: $0.90 with API
  Gateway). A submission 2 GETs and 2 PUTs, the attempt and then the pointer or certificate, about $1.08
  (spec: $1.50). A mark reads the claim, the course, a graduate's own certificate and the attempt, then
  writes the attempt and the certificate and deletes the pointer: 4 GETs and 2 PUTs, about $1.16 (spec:
  $1.50). The marking queue up to 3 GETs and a LIST, about $0.62; one attempt or its marking up to 3 GETs; a
  certificate 5 GETs (it, whether it counts, the job's run, and the teacher's and marker's claims for
  "removed"), about $0.20; the course page 3 GETs; every daily-built list 1 or 2 GETs. All under $2. The
  daily job lists `academy/courses/` (attempts and pointers included, per 1,000 keys) and `academy/
  certificates/`, and reads each course and each certificate once a day, so its reads grow with the
  certificates ever issued; that's about $0.40 a day per million certificates.
- **Bodies the spec leaves to the ticket:** a course is `{"title", "preparation"?, "pass", "set_size"?,
  "tasks": [{"kind": "check"|"work", "text", "answer"? | "answer_hash"?}]}`, `pass` a score from 1 to 100
  percent and `set_size` up to 20, by default the pool's size up to 20. An edit can send an answer check's
  `answer_hash` from `tasks` to keep it. An answer is `{"answer"}`. A mark is `{"accepted": [positions],
  "comment"?}`: the handed-in tasks it accepts; the score is right answer checks plus accepted work out of
  the set, in whole percent. Bodiless `PUT`s (`retirement`, `enrolments/self`, `submission`, `revocation`) and
  `POST .../attempts` take an empty body or `{}`.
- **Answer checks** are hashed with SHA-256, salted with the course ID, after trimming spaces at both ends,
  and compared case-sensitively. The student's answer to a check is stored only as its hash too, so neither
  `GET .../attempts/{attempt}` nor marking ever shows it; marking shows `correct: true|false`.
- **Choices the spec doesn't make:** retired courses keep their place among a teacher's 5, since there's no
  delete; the 7 days run from the last attempt's start date, so a later attempt may start while an earlier
  one still waits for a mark; an attempt copies its teacher and pass mark with its tasks; only its student
  reads `GET .../attempts/{attempt}` (`not-a-party`), and the teacher and graduates use `marking`; nobody marks
  its own attempt, refused as `403` `not-a-graduate`; a decided attempt's mark, and a repeated submission,
  return what was decided after redoing the follow-up writes (pointer, certificate, pointer delete), which
  is how a write that failed half way is corrected; a second pass of a course holder keeps the first
  certificate.
- **What the daily job builds** (`sfab_academy::daily`, after standing, through the new
  `sfab_vouches::with_standing`): the catalogue without retired courses, the rest after standing newest
  first; each course's `enrolled` and `graduates` (holders of certificates that aren't revoked) as 1,000-ID
  pages; teacher pages for anyone who teaches or marked, `marks` counting the passes it marked, since the
  job never reads attempts; `world/academy/certificates/<certificate>.json` only for certificates that count;
  `world/agents/<id>/enrolments/` and `certificates/` as 1,000-ID pages; and last `world/academy.json`, the
  marker that the job has run. The course page's counts come from the teacher's page, so one GET more, and
  its "lineage numbers" are the teacher's. Lists with nothing in them have no file, which after the first
  run reads as an empty page, as for an unknown ID. `sfab_core::world` gains `page_files`, `read_page` and
  `rebuild_where`, the last so the per-agent pages never delete ticket 09's takedown index beside them.
- **Takedown** gains `Action::Folders`: the counter `identities/<id>/academy.json` names the teacher's
  courses, and each course's whole folder goes, `course.json` last, so others' enrolments and attempts in
  it don't outlive it. Enrolments, attempts, pointers and held certificates are found by a `Search` of all
  of `academy/`, which grows with every attempt; ticket 09's index can replace it. Certificates the agent
  issued stay, showing it `removed: true`.
- **Rights grow:** the API Lambda may delete under `academy/` (leaving a course, a marked attempt's
  pointer), and the daily job may list and read `academy/`. Story 2's "Architecture", "Daily world job"
  ("Permissions") and "Storage" tables should say so.
- **The smoke script** gains 18 requests in the full run (publish, read, edit, tasks, enrol, start, read,
  hand in, submit, certificate, marking queue, refused own mark, revoke, retire, leave, and the four
  daily-built reads) and 9 in `--production` (the four daily-built reads, an unknown course and certificate,
  and tasks, an attempt and the marking queue of an unknown course), for about 84 and 49. The owner still
  has to run it against the deployed API.
