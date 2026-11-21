# Run the drills and finish the story

Label: ready-for-human
Blocked by: 02, 07, 08, 09, 11, 12, 14, 15, 17-genesis-codes#06

The checks that "Done" requires in `spec.md` "Delivery", run by the owner on the deployed API, as
described in "Testing".

Story 2 closes only when issue 16 does too. 16 is built on `main`, where story 4's code already is
(`4-life-in-the-world/spec.md` "Launch checklist", step 2). The drills don't need
it, so it isn't on the `Blocked by` line.

## Scope

- The kill switch drill, both ways in: the stop topic, and the daily limit lowered then restored.
  Confirm the email reaches your phone. While the web ACL's default action is Block, a request gets WAF's plain
  `403`, not `problem+json`, and the `docs/aws-facts.md` row on WAF blocks moves from `tested (rule)` to `tested`.
  Moved from issue 01 on 2026-11-20.
- The alarm check: force each alarm with `aws cloudwatch set-alarm-state`, and confirm its email arrives.
- The warning alarm's metric: after the smoke run, the 50,000-call warning's metric, with the `ApiName`
  CDK gave it, shows those calls, as `aws cloudwatch describe-alarms` and `get-metric-statistics` can show,
  because forcing its state doesn't prove the name is the one API Gateway reports, and with
  `NOT_BREACHING` a wrong name leaves it silent. Filed from code review C103.
- The budget check: a $0.01 email has arrived for spend beyond the domain's own charge, the stop budget
  stops the API with its limit temporarily lowered and the stop email names the budget alert, and its figure
  matches the first real bill.
- The WAF check, and that the requests WAF blocked added no API Gateway or CloudFront charge, which issue 02's
  test, run before the deploy, couldn't check.
- The billing test in issue 02, with its result acted on. Met on 2026-11-17: no rejected request was billed, and
  the design stands.
- The takedown check, for an agent with every kind of data.
- The website check, and that both distributions show the Free flat-rate plan.
- The census check, a day after deploying.
- The federation check, against a fake second world with no host: a token, an expiry and an approved
  world, but not a published token, as `spec.md` "Testing" explains.
- That the lifecycle rules exist on the deployed bucket.
- The smoke script over every route.
- Confirm the registrant email is verified.
- The cleanup and the final constitution aren't here. They're done once for both stories, in the "Launch
  checklist" of `.stories/4-life-in-the-world/spec.md`.

## Done when

- All of the above have passed, and any problem found has a ticket or a fix.
- The one public launch stays separate, and waits for `.stories/4-life-in-the-world/` and gratinierer/sfab#3.
