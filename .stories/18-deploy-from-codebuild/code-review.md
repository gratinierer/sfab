# 18-deploy-from-codebuild: code review

Code reviews of this story's issues. The journal lists every pass: `code-review` runs of `/review-code`,
`check` rows for the `scripts/check` run before each review, and `walkthrough` passes of
`/address-code-review-findings`. The findings table lists what each run found and what became of it. Rows are never
deleted or renumbered: a fixed finding flips to `resolved`.

Statuses: `open` / `resolved` / `wontfix` / `superseded`.
Axes: `standards` / `spec` / `security`.
Conventions: `docs/agents/issue-tracker.md`, section "Code review file".

## Review journal

| Date | Kind | Reviewed | By | Findings | Notes |
|---|---|---|---|---|---|
| 2026-11-14T14:20:00+00:00 | check | @ ee85ba7 | scripts/check | — | green (baseline of the implement-story run, after `chmod go-w ~/.cache/aws`) |
| 2026-11-14T14:20:00+00:00 | code-review | 01, 02, 03, 04 @ e9ab916..ee85ba7 | claude-sonnet-5-5 (dispatched; code by claude-opus-5-5) | 9 new: 8 standards, 1 spec, 0 security | Reviewers are the author's family on a different model, as no other vendor is available. Only the `(#18)` commits are reviewed. The range starts before 9c489ec so that issues 01 and 03, from an earlier unlogged run, are covered. |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| C1 | resolved | standards | 01, cdk/sfab_config.py:121 | `deploy_account()` swallows the fixture's `ConfigError` and falls through to STS, so a later STS failure never names why the fixture the owner set was rejected ("Errors: never drop an error silently"). | 2026-11-14: found. 2026-11-14: fixed now. deploy_account() keeps the fixture's ConfigError and names it in the STS failure message; new test test_an_unloadable_fixture_is_named_when_sts_fails_too; tests/test_sfab_config.py passes. |
| C2 | resolved | standards | 01, cdk/sfab_config.py:59 | `REQUIRED_KEYS` (and `FIXTURE_ONLY_KEYS`, `OPTIONAL_KEYS`) are hand-written tuples, not derived from `Config`, so a new key takes a field and a list entry, against CODING_STANDARDS.md "a new key is one field of its `Config`". | 2026-11-14: found. 2026-11-14: fixed now. REQUIRED_KEYS and OPTIONAL_KEYS are derived from Config's fields (no default / default), with FIXTURE_ONLY_KEYS the one hand-named exception; new test test_the_key_lists_come_from_config; test_sfab_config, test_app, test_deploy_plan pass. |
| C3 | resolved | standards | 02, cdk/deployer_stack.py:33 | `BOOTSTRAP_ROLES` and the `cdk-hnb659fds-*` role names rely on AWS/CDK bootstrap behaviour but cite no `docs/aws-facts.md` row. | 2026-11-14: found. 2026-11-14: fixed now. The BOOTSTRAP_ROLES comment names the role pattern and points to docs/aws-facts.md "CodeBuild and CodeConnections", whose row records the four roles and names; comment only, test_deployer_stack passes. |
| C4 | resolved | standards | 01, 02, cdk/deploy_plan.py:28 | Possible Duplicated Code: the region `"us-east-1"` is defined separately in `deploy_plan.py:28`, `deployer_stack.py:21`, `stacks.py:42` and `sfab_config.py:32` (`SECRET_REGION`). | 2026-11-14: found. 2026-11-14: fixed now. REGION is defined once, in deployer_stack.py; stacks.py and deploy_plan.py take it from there (stacks imports deployer_stack, so not the other way). sfab_config.SECRET_REGION stays, since the loader can't import the CDK code, and test_the_secret_lives_in_the_region_sfab_deploys_to pins it equal. test_deployer_stack, test_deploy_plan, test_stacks, test_app pass. |
| C5 | open | standards | 01, cdk/deploy_plan.py:29 | Possible Primitive Obsession: the check mode travels to `scripts/deploy` as the strings `require-secret`/`fallback` next to the `Plan.require_secret` bool, two representations of one flag. | 2026-11-14: found. 2026-11-14: needs the user (d195e40): a judgement call I'd waive. scripts/deploy is a thin bash wrapper that can only read a printed string, and Plan.require_secret is that string's Python side; a shared type can't cross the process boundary. Only you can waive. |
| C6 | open | standards | 01, cdk/sfab_config.py:100 | Possible Duplicated Code: `load()` and `deploy_account()` each parse the same `SFAB_CONFIG` fixture, and the `environ`/`aws` defaulting preamble repeats in `load`, `deploy_account`, `load_for_check`, `plan` and `main`. | 2026-11-14: found. 2026-11-14: needs the user (d195e40): a judgement call I'd waive. The double parse happens only in app.py on a fixture run (deploy_account() then load()), a few-line TOML file, and the environ/aws defaulting is the usual injectable-default idiom; folding either would widen the loader's interface for no behaviour. Only you can waive. |
| C7 | open | standards | 01, cdk/sfab_config.py:85 | Possible Speculative Generality: `Aws.client(service, region=None)` has a `region` parameter only one caller uses. | 2026-11-14: found. 2026-11-14: needs the user (d195e40): a judgement call I'd waive. The region parameter is used: _read_secret passes SECRET_REGION, so the secret is read in us-east-1 whatever the credentials' region; it is a real need, not speculation. Only you can waive. |
| C8 | open | standards | 01, cdk/sfab_config.py:90 | Possible Divergent Change: `sfab_config.py` also carries the region lookup `Aws.region`, the least config-related of its jobs. | 2026-11-14: found. 2026-11-14: needs the user (d195e40): a judgement call I'd waive. Aws is the one stubbed seam for every AWS lookup the loader and deploy plan make (issue 01 names it); moving region() out would add a second seam the tests must stub. Only you can waive. |
| C9 | open | spec | 01, cdk/smoke.py:787 | With `--api-url` and no `--production`, `load_target` no longer loads the config, a narrowing of spec "the smoke script reads it from the same source as the rest". | 2026-11-14: found. 2026-11-14: needs the user (d195e40): a spec reading the story doesn't settle. Without --production and with --api-url the smoke script needs nothing from the config, so not loading it lets it run with no secret; the spec line says only where config comes from when it is read. Either confirm the narrowing (and record it in spec.md) or make load_target always load. A product decision. |
