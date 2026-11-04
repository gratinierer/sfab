# Build the world routes and the daily world job

Label: ready-for-agent
Blocked by: 01, 07, 09

Stage 2. `spec.md` "World".

## Scope

- `/world/constitution`, from `world/constitution.md`, compiled in with a `version`. Add the Terms
  from `spec.md`, and an owner-written placeholder for the rest, which the owner replaces before
  launch.
- `birth`'s link to the constitution and its statement about the Terms.
- The third Rust function, the daily world job, set up as `spec.md` "Architecture" describes: just after
  00:00 UTC, 900 seconds, reserved concurrency of 1, no retries, about 50 parallel S3 calls and writing only what changed,
  with its own permissions, writing
  `census.json` and pages of 1,000 IDs, skipping markers, and deleting any page beyond the last one. Issue 15 adds to it. An email-only alarm
  on its errors.
- The kill switch's fourth stop step, setting this job's reserved concurrency to 0, with its permission and
  the matching step in the restart list, added here so the kill switch never points at a function that
  isn't there.
- `/world/census` and `/world/residents`, paged with `?page=`, with `404` for a page past the last,
  and before the job's first run, with `type: not-built-yet`, which a unit test checks.
- These routes in `sfab.yaml`, with origin key `security`, and in the smoke script.

## Done when

- Unit tests cover the snapshot's paging, skipping markers and deleting surplus pages.
- CDK assertion tests cover the schedule, timeout, reserved concurrency and permissions, and the kill
  switch's right to change this job's concurrency. A unit test covers the fourth stop step.
