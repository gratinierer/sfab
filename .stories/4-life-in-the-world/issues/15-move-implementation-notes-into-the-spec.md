# Move the implementation notes into the spec

Label: ready-for-agent
Blocked by:

Tickets 02-09 each end with a `## Comments` note from their implementation on 2026-11-17, "for the next
`/review-story` to move into the spec", as their "Every ticket here" footer asks. None has been moved yet
(review finding R109), so `spec.md` still carries figures and rules the code doesn't follow, such as a move at
"3 PUTs and 3 GETs, about $1.97" in "Homes and quarters", where ticket 03 counts 4 to 5 GETs and 3 PUTs at about
$1.70, and `moved_at` both kept and set on moving out.

## Scope

- For each of `issues/03-homes-quarters-and-lanes.md` and `issues/done/02`, `04` to `09`, move every point of its
  2026-11-17 note into the `spec.md` section it concerns: cost figures, refusal types and their fields, storage
  keys and what they hold, takedown actions, the daily job's and the API Lambda's new rights, and the smoke
  script's request counts.
- Where a note says story 2's spec should say something, such as its "Architecture", "Daily world job"
  ("Permissions") and "Storage" tables, make that edit in `.stories/2-identity-api/spec.md` too.
- Change only what the notes record about the built code; a point that contradicts a recorded decision is a
  question for the owner, not an edit.
- Leave each note in place with a dated line saying it was moved.

## Done when

- Every point of the eight notes is in `spec.md` or story 2's spec, or listed for the owner as contradicting a
  decision.
- `spec.md` "Homes and quarters" gives the built move cost and one `moved_at` rule.
