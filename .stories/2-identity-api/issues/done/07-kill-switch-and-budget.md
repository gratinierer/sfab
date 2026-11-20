# Build the kill switch and daily limit

Label: ready-for-human
Blocked by: 01

The first line of defence: `spec.md` "Abuse and cost controls", "Kill switch" and the email list in
"Operations". See `docs/adr/0001-initial-architecture.md`. The topics and budgets are in the
base stack, from issue 01.

## Scope

- The second Rust binary: the first three stop steps, an email naming the trigger, the 5-minute check of API Gateway's `Count` since
  midnight UTC against the daily limit, and the daily spend check.
- Subscribing the function to the stop topic, so the stop budget from issue 01 takes effect.
- The 5-minute and daily schedules, and permissions.
- Email-only alarms on the alerts topic: 50,000 calls a day, and errors in this function.
- Writing the restart list from `spec.md` "Kill switch" as CLI commands in the docs.

## Done when

- The kill switch unit tests and the CDK assertion tests in `spec.md` "Testing" pass, including that the
  50,000-call warning can't reach the stop topic.
- The owner has deployed the API stack, in issue 01's first deploy, because this container has no AWS credentials. The
  drills themselves are in issue 10.

## Comments

- 2026-11-08: relabelled `ready-for-human` (review finding R183): the kill switch (`cdk/kill_switch.py`, `lambda/killswitch/`), the budgets and `docs/restart-after-a-stop.md` are built and code-reviewed (`code-review.md`, 2026-11-08); what remains is the owner's deploy.
- 2026-11-20: closed. Both Done when items are met: the kill switch's unit tests and CDK assertion tests passed in
  `scripts/check` on `3522f41`, and the owner's first deploy brought it with the API stack (`SfabApi`, first from
  `b46e588`, then from `production` in build `sfab-deploy:41dbb6a3-c8c5-4da1-a76c-60e84a022a94`). Code review covers
  its last code commit, `6428761`, in the 2026-11-08 rounds ending at `ef30c3b`, with no finding open; the later
  `ee85ba7` changed only `docs/restart-after-a-stop.md` and was reviewed in story 18. The drills are issue 10's.
