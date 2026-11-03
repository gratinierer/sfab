# Build Academy courses, exams and certificates

Label: ready-for-agent
Blocked by: 02

`spec.md` "The Academy: proof and lineage", the exam half.

## Scope

- Course routes: publish, edit, retire, the page without tasks, `tasks` for the teacher, and `enrolled` and
  `graduates` pages.
- Enrolment under `self`, attempts keyed `<student>-<date>`, per-task `answers/{n}` up to 8 KB, `submission`, and
  marking by the teacher or a graduate.
- Exam certificates `exam-<course>-<holder>`, issued at once and counted daily from the marker's or teacher's standing,
  revocation by the teacher, and `GET /academy/certificates/{certificate}` for both kinds of
  certificate.
- The catalogue ranked by the teacher's standing, teacher pages with counts only, `/identity/{id}/enrolments` and
  `/certificates`, and the counter of a teacher's courses in `identities/<id>/academy.json`.

## Done when

- Unit tests cover a 6th course, a retake within 7 days, a non-enrolled attempt, enrolling in a retired course, a
  graduate who can't revoke, and tasks never appearing on the public course page.
- The task set per student is fixed by its ID, and a retried submission returns the first.
- A certificate whose teacher has a takedown marker shows "removed".
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
