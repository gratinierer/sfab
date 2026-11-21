# Run the drills and finish the story

Blocked by: 02, 07, 08, 09, 11, 12, 14, 15, 21, 17-genesis-codes#06

The checks that "Done" requires in `spec.md` "Delivery", run by the owner on the deployed API, as
described in "Testing".

Story 2 closes only when issue 16 does too. 16 is built on `main`, where story 4's code already is
(`4-life-in-the-world/spec.md` "Launch checklist", step 2). The drills don't need
it, so it isn't on the `Blocked by` line.

## Scope

- The kill switch drill, both ways in: the stop topic, and the daily limit lowered then restored. Run on the
  kill switch with issue 21's fix deployed: the stop must turn WAF to Block by itself, with no
  `ValidationException` in its log, since the 2026-11-21 drill found the WAF step failing.
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

## Comments

- 2026-11-21, run with the owner on the deployed API (`production` at `6f35461`, then `499b0b5` and `1410d51`):
  - **Kill switch drill, stop topic:** the first run found the WAF step failing on the web ACL's empty
    description, filed and fixed as issue 21; rerun on the fix, the kill switch set WAF to Block, both Lambdas to
    0 and disabled the endpoint itself, CloudFront answered WAF's plain `403` and the keyed `execute-api` request
    the disabled endpoint's `problem+json` `403`; the email arrived; restart list, then smoke 101/101.
  - **Kill switch drill, daily limit:** `DAILY_CALL_LIMIT` lowered to 100 (`499b0b5`) and deployed; the next
    5-minute check stopped the API the same way, and the email arrived; restored (`1410d51`), deployed, restart
    list, then smoke 101/101. The two commits cancel out, so they need no code review.
  - **Alarms:** all four forced to `ALARM`; every email arrived. The warning alarm watches `ApiName=Api`, the
    API's real name, whose metric showed the smoke run's 311 calls on 2026-11-20.
  - **Budgets:** the check budget's $0.01 email arrived. The stop budget's own email, and its figure against the
    first real bill, were left out by the owner's choice: the daily spend check reads the same budget and stops
    the API without SNS, and the check budget proved Budgets can publish. The owner keeps both, and the federation
    token's expiry, as later checks outside the repo.
  - **WAF check:** passed at request 78 with WAF's plain `403`; API Gateway's `Count` showed only the requests
    before the block, and the Free plan has no per-request charge.
  - **Billing test:** met on 2026-11-17, as above.
  - **Takedown check:** an agent with identity, claim, contacts, home, vouches, a course and attempt, a Square
    notice, `aboutme` and vitals, taken down 209 s after its birth: `self` `401`, `aboutme` `404`, the resent
    birth `403` `invalid-genesis-code`, only the claim marker left; a bulk `window` dry run listed and removed
    nothing.
  - **Website:** both pages and their links load over HTTPS, 30 requests in a row all `200`; both distributions
    on their own Free plans (`done/12`).
  - **Census:** `/world/census` 8 residents, the five founders and three test agents, no markers;
    `/world/residents?page=` past the last answers `404`. The job's log shows no `403`, so `docs/aws-facts.md`'s
    `memory` row on prefix-conditioned `ListBucket` stays `memory`, with that noted.
  - **Federation:** the smoke runs got a token for `smoke-federation.<domain_name>`; an approved world appearing
    after a deploy was left out by the owner's choice, since the same deploy path served `federation/world.yaml`'s
    new name, and its expiry is one of the owner's later checks.
  - **Lifecycle rules:** all six on the identities bucket.
  - **Smoke script:** five full runs, all 101/101 but one, whose `GET /world/residents` met the stage redeploy
    seconds after a restart and passed on the rerun.
  - **Registrant email:** verified (`GetContactReachabilityStatus` `DONE`).
