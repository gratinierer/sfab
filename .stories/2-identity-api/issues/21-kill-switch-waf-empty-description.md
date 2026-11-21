# Let the kill switch's WAF step update a web ACL with no description

Label: ready-for-agent
Blocked by:

Found by issue 10's kill switch drill on 2026-11-21: publishing to the stop topic set both Lambdas' reserved
concurrency to 0 and disabled the `execute-api` endpoint, but the WAF step failed every time with
`wafv2 UpdateWebACL: unhandled error (ValidationException)`, so agents kept getting `problem+json` errors instead
of WAF's plain `403`. The web ACL has no description, which `GetWebACL` returns as `""`, and `Step::Waf` in
`lambda/killswitch/src/main.rs` sends it back with `.set_description(acl.description)`; WAF requires a description
of at least 1 character. The same `UpdateWebACL` without `Description` is accepted, which finished the drill's stop
by hand. `docs/restart-after-a-stop.md` step 5's `jq` keeps `Description: ""` too, since it drops only `null`.

The drill also showed that a disabled `execute-api` endpoint answers the keyed request with the API's own
`problem+json` `403` (`origin-key-replaced`), not a plain `403`, so the spec's and `docs/aws-facts.md`'s "plain
`403`" is wrong.

## Scope

- `Step::Waf` sends `Description` only when it isn't empty, with a unit test on the request it builds, or on the
  function that decides, for an ACL whose description is `""`.
- `docs/restart-after-a-stop.md` step 5's `jq` also drops an empty string, and nothing else in it changes.
- `docs/aws-facts.md`: the `disableExecuteApiEndpoint` row says the disabled endpoint's `403` goes through the
  stage's gateway responses, as `problem+json`, `tested` on 2026-11-21; and a row for `UpdateWebACL` rejecting an
  empty `Description`, `tested`.
- `spec.md` "Testing", the smoke script's bullet ("a stopped API, whose disabled endpoint answers a plain `403`,
  can't pass") and `cdk/smoke.py`'s `_execute_api` docstring stop saying so: the disabled endpoint's answer is the
  same `origin-key-replaced` `403`, so those two checks can't tell a stopped API, which the smoke script still
  meets as WAF's block or the Lambdas' refusals.

## Done when

- The unit test fails before the fix and passes after it.
- After the deploy that carries it, issue 10's drill 1 stops WAF through the kill switch itself, with no
  `ValidationException` in its log.
