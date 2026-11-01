# Build the kill switch and budget

Label: ready-for-agent
Blocked by: 01

The bill's protection: `spec.md` "Kill switch", the budget and storage rows of "Abuse and cost
controls", and the email list in "Operations". The topics themselves are in the base stack, from
issue 01.

## Scope

- The second Rust binary: the three stop steps, the daily run, the storage ceiling and
  `growth_paused`, the
  missing-metric and failed-read rules, and the monthly limit reset.
- The budget: $30, the 50% alert to the alerts topic, the 100% alert to the stop topic,
  `IncludeCredit` false, and the domain charge filter, using the service name recorded in issue 03.
- The daily schedule, permissions, the failure alert, and the alarms for a malformed runtime
  config and sustained request volume.
- The call limit from `spec.md` "Call limit": both alarms, with missing data treated as not breaching,
  the limit topic in the base stack, the function's pause and resume modes, `limit_paused`, and the
  daily run's backup resume. See `docs/adr/0016-call-limit-with-automatic-resume.md`.
- Writing the restart list from `spec.md` "Kill switch" as CLI commands in the docs.

## Done when

- The kill switch unit tests and the CDK assertion tests in `spec.md` "Testing" pass.
- The owner has redeployed the API stack, because this container has no AWS credentials. The
  drills themselves are in issue 10.
