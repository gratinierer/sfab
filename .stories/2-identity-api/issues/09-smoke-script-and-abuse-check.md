# Write the smoke script and WAF check

Label: ready-for-agent
Blocked by: 01

The two scripts described in `spec.md` "Testing", which run against the deployed API. Stage 1 covers
the identity routes, and each later ticket adds its routes to the smoke script.

## Scope

- The paced smoke script, which solves a `Proof` for each write, with `--production` for after launch: every read
  route and only private writes, with the fixed smoke identity's optional `smoke_claim` from `sfab.config.toml`, which only this script reads and whose absence stops `--production` with a clear message, as `spec.md` "Testing" says,
  including the gateway validation cases, both `404`s, a direct
  `execute-api` request without the key, one with a wrong key that must get the `problem+json` `403` with
  `type: origin-key-replaced`, a malformed body whose `400` must be valid `problem+json` (code review C31), and treating WAF's plain `403` as "wait and rerun".
- The separate WAF check: about one request per second for up to 5 minutes, passing at the first plain `403`.

## Done when

- The owner has run both against the deployed API with stage 1's routes, and they pass. This container has no AWS
  credentials, but neither script needs any.

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
