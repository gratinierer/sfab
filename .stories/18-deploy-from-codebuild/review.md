# 18-deploy-from-codebuild: review

Review record for this story. The journal lists every pass over it — reviews
and walkthroughs, distinguished by `Kind`; the
findings table lists what each pass found and what became of it. Rows are
never deleted — a fixed finding flips to `resolved`, it does not disappear.

Statuses: `open` / `resolved` / `wontfix` / `superseded`.
Axes: `factual` / `consistency` / `completeness` / `convention` / `staleness`.
Conventions: `docs/agents/issue-tracker.md`, section "Review file".

## Review journal

| Date | Kind | Reviewed | By | Findings | Notes |
|---|---|---|---|---|---|
| 2026-11-09T16:03:00+00:00 | review | cca647d | claude-fable-5-1 (dispatched; story authored by claude-opus-5-5, orchestrated by claude-opus-5-5, auto round 1) | 14 new, 0 resolved, 14 open | first review |

## Findings

| # | Status | Axis | Affects | Description | Comments |
|---|---|---|---|---|---|
| R1 | open | completeness | 02, 05, spec.md | `production` is cut "from `main`", but `main` holds only `.stories`, `AGENTS.md`, `GLOSSARY.md` and `docs` (`git ls-tree main`); all code is on `2-identity-api-birth-evolve-self-and-reclaim`, so a build from `production` finds no buildspec or `scripts/deploy`, and no issue owns merging into `main` or names `production`'s source. | 2026-11-09: found. 2026-11-09 (auto walkthrough): needs the user (163a5de): which branch `production` is cut from is the owner's call: (a) merge story 2's branch into `main` before the first build, so `production` follows `main` as Clarification 3 says, or (b) cut `production` from `2-identity-api-birth-evolve-self-and-reclaim` until that merge, and say so in issue 05 and `spec.md`. |
| R2 | resolved | completeness | 01, 02, spec.md | The first `SfabDeployer` deploy can't pass `scripts/check`: its generators exit on `ConfigError` (`generate_website.py:54-55`, `generate_public_openapi.py:155-156`) while the secret doesn't exist yet, and issue 01 removes the fixture fallback; only `cdk/app.py` gets the "synth only `SfabDeployer`" rule, and setup step 2's raw `cdk deploy SfabDeployer` path (venv, CLI) isn't specified. | 2026-11-09: found. 2026-11-09 (auto walkthrough): fixed: the first deploy is `scripts/deploy SfabDeployer`, which with no loadable config accepts only `SfabDeployer` and runs `scripts/check` without its generate step (`spec.md` "Synth without the config" and setup step 2, issue 02 scope and Done when). |
| R3 | resolved | completeness | 01, 02 | `stacks.build` takes the env from `config.account_id` (`stacks.py:558`) and issue 01 puts the STS lookup inside `load()`, so when the secret is missing nothing supplies `SfabDeployer`'s account; issue 02 needs its own lookup (STS or `CDK_DEFAULT_ACCOUNT`) and a test. | 2026-11-09: found. 2026-11-09 (auto walkthrough): fixed: issue 01 puts the STS lookup in its own `deploy_account()`, separate from `load()`, and issue 02 targets `SfabDeployer` with it, so that stack synthesizes when the config doesn't load, with a test. |
| R4 | open | factual | 03, spec.md | With dynamic references, "replace the value in the secret, then start a build" rotates nothing: the template doesn't change, so CloudFormation leaves the `ApiKey`, the CloudFront header and `IP_HASH_KEY` as they were (reviewer's understanding, unchecked; no `docs/aws-facts.md` row), and `stacks.py:397-399`'s "a changed value replaces the key" relies on the value being in the template; issue 03 should confirm this and give a rotation mechanism. | 2026-11-09: found. |
| R5 | open | completeness | 04 | Issue 04's sweep and review list miss affected places: `.stories/4-life-in-the-world/spec.md:295` and `issues/10-launch-checklist.md:16`, `.stories/2-identity-api/issues/03-*.md:16,22`, `issues/09-*.md:12`, `federation/world.yaml:2`, `cdk/tests/conftest.py`, `test_stacks.py`, `test_smoke_args.py`; story 4 gets no `/review-story` line; and "outside `.stories/done/`" misses `<story>/issues/done/` and immutable `review.md` rows. | 2026-11-09: found. |
| R6 | open | completeness | 04, spec.md | `2-identity-api#03` (prepare the account, `cdk bootstrap`, write the TOML) blocks `2#01` but story 18 never mentions it and it isn't `Blocked by` story 18, so the owner could create the TOML this story says will never exist. | 2026-11-09: found. |
| R7 | open | completeness | 01 | Removing `sfab.config.example.toml` breaks `cdk/tests/test_app.py:35` (`test_synth_refuses_the_copied_example`), and issue 01's file list misses `cdk/waf_check.py:10` and `cdk/smoke.py:7`, whose docstrings say "Needs no AWS credentials", no longer true once they read the secret. | 2026-11-09: found. |
| R8 | open | consistency | 01, spec.md | `account_id` leaves the secret's keys while the fixture "keeps its `000000000000`" under a parser that refuses unknown keys; the issue doesn't say whether `account_id` is valid in the fixture only or how the fixture supplies the account. | 2026-11-09: found. |
| R9 | open | completeness | 01, 04 | Issue 01 makes `scripts/check` without credentials fail instead of falling back to the fixture, but agent containers have no AWS credentials and rely on that fallback (`scripts/check:8-13`, story 2 "Delivery"); no issue updates `AGENTS.md`, the devcontainer or the Delivery text so agents set `SFAB_CONFIG`. | 2026-11-09: found. |
| R10 | open | completeness | 02, spec.md | New accounts may start with a CodeBuild concurrency quota of 0 ("Cannot have more than 0 builds in queue", forum reports, undocumented); no issue adds a quota check or support case beside story 2's Lambda quota prerequisite, nor an unconfirmed `docs/aws-facts.md` row. | 2026-11-09: found. |
| R11 | open | completeness | 02, spec.md | The failure alert lives in `SfabBase`, so every build before `SfabBase` exists, including all builds before the domain is bought, fails silently; the spec presents the alert without stating this gap. | 2026-11-09: found. |
| R12 | open | consistency | spec.md, 04 | Setup step 5 doesn't say that story 2's step 3 (the registrar service name in the budget filter) is a code change that must be committed and moved onto `production` before the `SfabBase` build, nor that `production` moves forward between first-deploy steps. | 2026-11-09: found. |
| R13 | open | factual | 02 | "CI and the build read the versions from one place" needs `ci.yml` restructured: it pins tools as inline `env:` values (`UV_VERSION`, `CARGO_LAMBDA_VERSION`, `ZIGLANG_VERSION`, `CARGO_AUDIT_VERSION`) with no CDK CLI pin; issue 02 should name the shared file so its Done when can be checked. | 2026-11-09: found. |
| R14 | open | convention | 02, 05 | Issue 05 has no `Blocked by` though it needs R1's merge into `main` first, and issue 02's deployed proof needs 05 but goes to `2-identity-api#01`, which reaches 02 and 03 only through 04 rather than naming them. | 2026-11-09: found. |
