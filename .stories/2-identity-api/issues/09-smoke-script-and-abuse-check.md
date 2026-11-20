# Write the smoke script and WAF check

Label: ready-for-human
Blocked by: 01

The two scripts described in `spec.md` "Testing", which run against the deployed API. The smoke script
covers every route.

## Scope

- The paced smoke script, which solves a `Proof` for each write, with `--production` for after launch: every read
  route and only private writes, with the fixed smoke identity's optional `smoke_claim` from the config secret `sfab/config`, which only this script reads and whose absence stops `--production` with a clear message, as `spec.md` "Testing" says,
  including the gateway validation cases, both `404`s, a direct
  `execute-api` request without the key, one with a wrong key that must get the `problem+json` `403` with
  `type: origin-key-replaced`, a malformed body whose `400` must be valid `problem+json` (code review C31), and treating WAF's plain `403` as "wait and rerun".
- The separate WAF check: about one request per second for up to 5 minutes, passing at the first plain `403`.
- Later tickets extend the smoke script with their routes: issues 11 (world and census, with the
  census's `404` before the first daily run), 13 (profile, friends and blocks), 14 (messages) and 15
  (federation), each saying so in its scope, and `.stories/4-life-in-the-world/` "Routes" adds the
  `403` `no-standing` and unknown-ID cases. Filed from code review C96.

## Done when

- The owner has run both against the deployed API with every route, and they pass. Without `--api-url`, both
  build the API's URL from the config secret `sfab/config`'s `domain_name`, so the owner runs them with `aws login`
  credentials or passes `--api-url`; only the smoke script's `--production` also needs the secret, for its
  `smoke_claim`. The smoke run births with exempt genesis codes from one IP
  without hitting `409 too-soon` (`17-genesis-codes#06`).
- On that first run, the response to the `execute-api` request without the key is recorded in the
  `docs/aws-facts.md` row on a missing API key, and if it's the `problem+json` `403` with
  `type: origin-key-replaced`, the smoke check is tightened to require it, so a stopped or blocked API
  can't pass it. Filed from code review C95.

## Comments

- 2026-11-07: built, with no AWS account yet, so neither script has run against a deployed API. Both are
  Python on the standard library, in `cdk/` beside the config parser they reuse: `cdk/smoke.py` and
  `cdk/waf_check.py`, run through `scripts/smoke` and `scripts/waf-check`, with `cdk/api_client.py` solving
  proofs (checked against the API description's worked example) and reading responses. Their tests run
  against a fake of the deployed API in `cdk/tests/fakes.py`, never the network. The smoke script takes
  `--execute-api <stage URL>`, the API stack's `Endpoint` output that `cdk deploy` prints, which a CDK test
  pins. `smoke_claim` holds `<id>.<claim>`, as `Authorization` sends it, because the script needs the ID
  too; the example file says so. Exit codes: 0 pass, 1 fail, 2 usage or config, 3 blocked by WAF.
- The full run sends 19 requests (about 75 seconds), `--production` 13. The maximum-size escaped body
  expects the Lambda's `400` whose title names "32 KB", which proves it passed the gateway. A request to
  `execute-api` without the key passes on any `403`, because `docs/aws-facts.md` has only memory for which
  gateway response it raises; with a wrong key it needs `problem+json` with `type: origin-key-replaced`.
- 2026-11-08: relabelled `ready-for-human` (review finding R183): the scripts are built and code-reviewed;
  what remains is the owner's run against the deployed API.
- 2026-11-14: since `.stories/18-deploy-from-codebuild/`, the 2026-11-07 note is out of date in two places. Deploys
  run in CodeBuild, so the API stack's `Endpoint` output is in the build log and the stack's CloudFormation outputs,
  not printed by a local `cdk deploy`. `sfab.config.example.toml` is gone, and `smoke_claim`'s `<id>.<claim>` form
  is documented in `spec.md` "The config secret".
- 2026-11-16: the C95 item is done in code ahead of the first run. On a throwaway API imported from `sfab.yaml`
  (issue 01's comment of today), a request without the key got `INVALID_API_KEY`'s `403` `origin-key-replaced`,
  but as `application/json` until its template was keyed `application/problem+json`, which `sfab.yaml` now does.
  The smoke check now requires that `problem+json` `403` without the key too, so a stopped API can't pass it. The
  first run against the deploy still confirms it and records it in the `docs/aws-facts.md` row.
- 2026-11-20: first runs against the deployed API (`production` at `3522f41`, with story 4's routes and the five
  founders). `scripts/smoke --execute-api https://xq2b71gpph.execute-api.us-east-1.amazonaws.com/v1/`: 99 of 100
  checks passed, and it birthed test agent `i5splwu5cewn` with a `--no-ip-limit` code and no `409 too-soon` (9 codes
  left). The one failure is the script's, not the API's: `vouched-by while unpublished` (`cdk/smoke.py:761`, from
  `4-life-in-the-world` `7d07f91`) expects `200`, but before the daily world job's first run the public
  `vouched-by` answers `404 not-built-yet`, as `.stories/4-life-in-the-world/spec.md` "Vouches" says; the check
  above it, `read own vouched-by`, already allows that. `execute-api` without the key and with a wrong key both got
  the `problem+json` `403` `origin-key-replaced`, recorded in the `docs/aws-facts.md` row on a missing API key.
  `scripts/waf-check` passed: request 83 got WAF's plain `403`, right after the smoke run's requests, so about 100
  in 5 minutes. Still open: a smoke run with every check passing.
- 2026-11-20: `4314be5` lets `vouched-by while unpublished` accept `404 not-built-yet`, as `read own vouched-by`
  does, test first: the fake API answered `200` there, so it now answers `not-built-yet`, and new tests cover the
  empty page after the job's first run and refuse anything else. The rerun of `scripts/smoke` passed all 100 checks
  and birthed test agent `xwehbejahu5b` (8 codes left). Every Done when item is met; `4314be5` touches `cdk/`, so
  the issue waits for `/review-code` before closing.
