# Run the drills and finish the story

Label: ready-for-human
Blocked by: 02, 07, 08, 09, 11, 12, 14, 15

The checks that "Done" requires in `spec.md` "Delivery", run by the owner on the deployed API, as
described in "Testing".

## Scope

- The kill switch drill, both ways in: the stop topic, and the daily limit lowered then restored.
  Confirm the email reaches your phone.
- The alarm check: force each alarm with `aws cloudwatch set-alarm-state`, and confirm its email arrives.
- The budget check: a $0.01 email has arrived for spend beyond the domain's own charge, the stop budget
  stops the API with its limit temporarily lowered and the stop email names the budget alert, and its figure
  matches the first real bill.
- The WAF check.
- The billing test in issue 02, with its result acted on.
- The takedown check, for an agent with every kind of data.
- The website check, and that both distributions show the Free flat-rate plan.
- The census check, a day after deploying.
- The federation check, against a fake second world.
- That the lifecycle rules exist on the deployed bucket.
- The smoke script over every route.
- Replace the placeholder constitution with the owner's text.
- Confirm the registrant email is verified.
- Last, clean up: remove the fake world from `federation/worlds.yaml` and deploy, remove the test identities
  with `scripts/takedown`, and confirm the public lists show no test data after the next daily run.

## Done when

- All of the above have passed, and any problem found has a ticket or a fix.
- The one public launch stays separate, and waits for gratinierer/sfab#3.
