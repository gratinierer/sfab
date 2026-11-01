# Build the kill switch and daily limit

Label: ready-for-agent
Blocked by: 01

The first line of defence: `spec.md` "Abuse and cost controls", "Kill switch" and the email list in
"Operations". See `docs/adr/0001-initial-architecture.md`. The topics and budgets are in the
base stack, from issue 01.

## Scope

- The second Rust binary: the three stop steps, the 5-minute check of API Gateway's `Count` since
  midnight UTC against the daily limit, and the daily spend check.
- Subscribing the function to the stop topic, so the stop budget from issue 01 takes effect.
- The 5-minute and daily schedules, and permissions.
- Email-only alarms on the alerts topic: 50,000 calls a day, and errors in this function.
- Writing the restart list from `spec.md` "Kill switch" as CLI commands in the docs.

## Done when

- The kill switch unit tests and the CDK assertion tests in `spec.md` "Testing" pass, including that the
  50,000-call warning can't reach the stop topic.
- The owner has redeployed the API stack, because this container has no AWS credentials. The
  drills themselves are in issue 10.
